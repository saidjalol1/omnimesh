use crate::config::OmnimeshMode;
use crate::config::modes::TransportType;
use crate::envelope::SignedEnvelope;
use crate::runtime::transport::config::TransportConfig;
use crate::runtime::transport::mock::MockTransport;
use crate::runtime::transport::quic::QuicTransport;
use crate::runtime::transport::tcp::TcpTransport;
use crate::runtime::transport::interface::{Transport, DEFAULT_PAYLOAD_CAPACITY};

/// Transport layer facade that provides a unified interface over different transport implementations.
///
/// This layer acts as a factory for creating transport instances based on the runtime mode
/// and provides a consistent interface for the rest of the OMNI-MESH runtime.
#[derive(Debug)]
pub struct TransportLayer {
    transport: Box<dyn Transport>,
    config: TransportConfig,
    #[allow(dead_code)]
    routing: std::sync::Arc<crate::runtime::RoutingTable>,
}

impl TransportLayer {
    /// Creates a new transport layer for the given runtime mode.
    ///
    /// The transport type is automatically selected based on the mode's transport_type() method.
    /// Each transport is initialized with appropriate configuration.
    pub fn new(mode: &OmnimeshMode) -> Result<Self, String> {
        let config = TransportConfig::default();
        let routing = std::sync::Arc::new(crate::runtime::RoutingTable::new());

        let transport: Box<dyn Transport> = match mode.transport_type() {
            TransportType::Mock => Box::new(MockTransport::new(routing.clone())),
            TransportType::Tcp => Box::new(TcpTransport::new(config.clone(), routing.clone())?),
            TransportType::Quic => Box::new(QuicTransport::new(config.clone(), routing.clone())?),
        };

        if !matches!(mode.transport_type(), TransportType::Mock) {
            let gossip_addr = "0.0.0.0:8888".parse().unwrap();
            routing.clone().start_gossip_task(1000, gossip_addr);
        }

        Ok(TransportLayer { transport, config, routing })
    }

    /// Creates a new transport layer with custom configuration.
    ///
    /// This allows overriding the default network configuration for specific deployments.
    pub fn with_config(mode: &OmnimeshMode, config: TransportConfig) -> Result<Self, String> {
        let routing = std::sync::Arc::new(crate::runtime::RoutingTable::new());
        let transport: Box<dyn Transport> = match mode.transport_type() {
            TransportType::Mock => Box::new(MockTransport::new(routing.clone())),
            TransportType::Tcp => Box::new(TcpTransport::new(config.clone(), routing.clone())?),
            TransportType::Quic => Box::new(QuicTransport::new(config.clone(), routing.clone())?),
        };

        if !matches!(mode.transport_type(), TransportType::Mock) {
            let gossip_addr = "0.0.0.0:8888".parse().unwrap();
            routing.clone().start_gossip_task(1000, gossip_addr);
        }

        Ok(TransportLayer { transport, config, routing })
    }

    /// Initializes the transport layer.
    ///
    /// This performs any necessary setup and logs the transport type being used.
    pub fn initialize(&self) -> Result<(), String> {
        println!("Initializing transport layer: {}", self.transport.kind());
        Ok(())
    }

    /// Returns the kind of transport being used.
    pub fn kind(&self) -> &'static str {
        self.transport.kind()
    }

    /// Attempts to receive an envelope from the transport.
    ///
    /// This delegates to the underlying transport implementation.
    pub fn receive(&self) -> Option<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        self.transport.receive()
    }

    /// Sends an envelope through the transport.
    ///
    /// This delegates to the underlying transport implementation.
    pub fn send(&self, envelope: &SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        self.transport.send(envelope)
    }

    /// Returns the current transport configuration.
    pub fn config(&self) -> &TransportConfig {
        &self.config
    }
}