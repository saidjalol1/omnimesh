use crate::envelope::SignedEnvelope;
use crate::runtime::transport::config::TransportConfig;
use crate::runtime::transport::cert::{CertificateConfig, CertificatePair};
use crate::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};
use crate::runtime::transport::common::{TransportUtils, errors, logging};
use crate::config::modes::layer_kinds;
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

use quinn::{Endpoint, ServerConfig, ClientConfig};

/// Dummy certificate verifier for self-signed certificates in dev mode
#[derive(Debug)]
struct DummyVerifier;
impl rustls::client::danger::ServerCertVerifier for DummyVerifier {
    fn verify_server_cert(
        &self,
        _end_entity: &rustls::pki_types::CertificateDer<'_>,
        _intermediates: &[rustls::pki_types::CertificateDer<'_>],
        _server_name: &rustls::pki_types::ServerName<'_>,
        _ocsp_response: &[u8],
        _now: rustls::pki_types::UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &rustls::pki_types::CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::ED25519,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
        ]
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TransportStats {
    pub messages_sent: u64,
    pub messages_received: u64,
    pub send_failures: u64,
    pub backpressure_events: u64,
    pub reconnections: u64,
}

struct SendRequest {
    envelope: SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>,
    addr: std::net::SocketAddr,
}

#[derive(Debug)]
pub struct QuicTransport {
    kind: &'static str,
    runtime: Arc<tokio::runtime::Runtime>,
    rx: Arc<std::sync::Mutex<mpsc::UnboundedReceiver<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>>>,
    config: TransportConfig,
    #[allow(dead_code)]
    certs: Arc<CertificatePair>,
    routing: Arc<crate::runtime::RoutingTable>,
    send_buffer: Arc<Mutex<mpsc::Sender<SendRequest>>>,
    stats: Arc<Mutex<TransportStats>>,
}

impl QuicTransport {
    pub fn new(config: TransportConfig, routing: Arc<crate::runtime::RoutingTable>) -> Result<Self, String> {
        let _ = rustls::crypto::ring::default_provider().install_default();
        let runtime = TransportUtils::create_runtime()?;

        let cert_config = CertificateConfig::default();
        let certs = Arc::new(CertificatePair::generate_self_signed(cert_config)?);

        if !certs.is_valid() {
            return Err(errors::INVALID_CERTIFICATES.to_string());
        }

        let cert_der = rustls::pki_types::CertificateDer::from(certs.cert_der().to_vec());
        let key_der = rustls::pki_types::PrivateKeyDer::try_from(certs.key_der().to_vec())
            .map_err(|e| format!("Invalid private key: {:?}", e))?;

        let mut server_crypto = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(vec![cert_der], key_der)
            .map_err(|e: rustls::Error| e.to_string())?;
        
        server_crypto.alpn_protocols = vec![b"omnimesh".to_vec()];
        let quic_server_config = quinn::crypto::rustls::QuicServerConfig::try_from(server_crypto)
            .map_err(|e| e.to_string())?;
        let server_config = ServerConfig::with_crypto(Arc::new(quic_server_config));

        let listen_addr = config.quic_listen_addr;
        let endpoint = runtime.block_on(async {
            Endpoint::server(server_config, listen_addr)
        }).map_err(|e| format!("QUIC bind failed: {}", e))?;

        logging::quic_endpoint_initialized(listen_addr);

        let (tx, rx) = mpsc::unbounded_channel();
        let (send_tx, mut send_rx) = mpsc::channel::<SendRequest>(1000); // Bounded for backpressure
        let endpoint_clone = endpoint.clone();
        let stats = Arc::new(Mutex::new(TransportStats::default()));

        let runtime = Arc::new(runtime);

        let transport = QuicTransport {
            kind: layer_kinds::QUIC_TRANSPORT,
            runtime: runtime.clone(),
            rx: Arc::new(std::sync::Mutex::new(rx)),
            config: config.clone(),
            certs,
            routing: routing.clone(),
            send_buffer: Arc::new(Mutex::new(send_tx)),
            stats: stats.clone(),
        };

        // Start send worker with flow control
        let endpoint_send = endpoint.clone();
        let stats_clone = stats.clone();
        let runtime_clone = runtime.clone();
        runtime_clone.spawn(async move {
            while let Some(req) = send_rx.recv().await {
                let mut stats_guard = stats_clone.lock().await;
                
                // Try to send with exponential backoff
                let mut retries = 0;
                let max_retries = 3;
                
                while retries < max_retries {
                    let mut client_crypto = rustls::ClientConfig::builder()
                        .dangerous()
                        .with_custom_certificate_verifier(Arc::new(DummyVerifier))
                        .with_no_client_auth();
                    client_crypto.alpn_protocols = vec![b"omnimesh".to_vec()];
                    let quic_client_config = quinn::crypto::rustls::QuicClientConfig::try_from(client_crypto).unwrap();
                    let client_config = ClientConfig::new(Arc::new(quic_client_config));

                    match endpoint_send.connect_with(client_config, req.addr, "localhost") {
                        Ok(connecting) => {
                            match connecting.await {
                                Ok(connection) => {
                                    match connection.open_uni().await {
                                        Ok(mut stream) => {
                                            let mut buf = [0u8; 2048];
                                            if let Ok(len) = req.envelope.serialize_into(&mut buf) {
                                                match stream.write_all(&buf[..len]).await {
                                                    Ok(_) => {
                                                        stats_guard.messages_sent += 1;
                                                        break;
                                                    }
                                                    Err(_) => {
                                                        stats_guard.send_failures += 1;
                                                        retries += 1;
                                                        
                                                        if retries < max_retries {
                                                            stats_guard.reconnections += 1;
                                                            tokio::time::sleep(tokio::time::Duration::from_millis(
                                                                100 * (1 << retries)
                                                            )).await;
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                        Err(_) => {
                                            stats_guard.send_failures += 1;
                                            retries += 1;
                                            
                                            if retries < max_retries {
                                                tokio::time::sleep(tokio::time::Duration::from_millis(
                                                    100 * (1 << retries)
                                                )).await;
                                            }
                                        }
                                    }
                                }
                                Err(_) => {
                                    stats_guard.send_failures += 1;
                                    retries += 1;
                                    
                                    if retries < max_retries {
                                        tokio::time::sleep(tokio::time::Duration::from_millis(
                                            100 * (1 << retries)
                                        )).await;
                                    }
                                }
                            }
                        }
                        Err(_) => {
                            stats_guard.send_failures += 1;
                            retries += 1;
                            
                            if retries < max_retries {
                                tokio::time::sleep(tokio::time::Duration::from_millis(
                                            100 * (1 << retries)
                                        )).await;
                            }
                        }
                    }
                }
            }
        });

        // Start accept loop
        let tx_clone = tx.clone();
        let stats_clone = stats.clone();
        let runtime_clone = runtime.clone();
        runtime_clone.spawn(async move {
            Self::accept_loop(endpoint_clone, tx_clone, stats_clone).await;
        });

        Ok(transport)
    }

    async fn accept_loop(
        endpoint: Endpoint, 
        tx: mpsc::UnboundedSender<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>,
        stats: Arc<Mutex<TransportStats>>
    ) {
        while let Some(conn) = endpoint.accept().await {
            let tx = tx.clone();
            let stats = stats.clone();
            tokio::spawn(async move {
                if let Ok(connection) = conn.await {
                    while let Ok(mut stream) = connection.accept_uni().await {
                        let tx = tx.clone();
                        let stats = stats.clone();
                        tokio::spawn(async move {
                            if let Ok(data) = stream.read_to_end(1024 * 1024).await
                                && let Ok(envelope) = SignedEnvelope::deserialize(&data) {
                                    if tx.send(envelope).is_ok() {
                                        let mut stats_guard = stats.lock().await;
                                        stats_guard.messages_received += 1;
                                    }
                                }
                        });
                    }
                }
            });
        }
    }

    /// Returns transport statistics
    pub fn stats(&self) -> TransportStats {
        self.runtime.block_on(async {
            *self.stats.lock().await
        })
    }
}

impl Transport for QuicTransport {
    fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        match self.rx.lock() {
            Ok(mut rx) => rx.try_recv().ok(),
            Err(_) => None,
        }
    }

    fn send(&self, envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        let connect_addr = self.routing.resolve(&envelope.header.recipient_did)
            .unwrap_or(self.config.quic_listen_addr);
        
        let req = SendRequest {
            envelope: *envelope,
            addr: connect_addr,
        };
        
        // Try to send with backpressure handling
        self.runtime.block_on(async {
            let send_buffer = self.send_buffer.lock().await;
            match send_buffer.try_send(req) {
                Ok(_) => Ok(()),
                Err(mpsc::error::TrySendError::Full(_)) => {
                    // Backpressure: buffer is full
                    let mut stats_guard = self.stats.lock().await;
                    stats_guard.backpressure_events += 1;
                    Err("Send buffer full - backpressure applied".to_string())
                }
                Err(mpsc::error::TrySendError::Closed(_)) => {
                    Err("Send channel closed".to_string())
                }
            }
        })
    }

    fn kind(&self) -> &'static str {
        self.kind
    }
}
