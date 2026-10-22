use crate::config::modes::OmniMeshConfig;
use crate::envelope::Did;
use crate::runtime::RuntimeLayer;
use crate::runtime::RuntimeStats;
use crate::runtime::delivery::DeliveryLayer;
use crate::runtime::logging::{LogEntry, LogLevel, Logger};
use crate::runtime::security::SecurityLayer;
use crate::runtime::storage::StorageLayer;
use crate::runtime::transport::TransportLayer;
use crate::runtime::transport::TransportConfig;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

#[derive(Debug)]
pub struct Runtime {
    self_did: Did,
    transport: TransportLayer,
    security: SecurityLayer,
    storage: StorageLayer,
    delivery: DeliveryLayer,
    stats: RuntimeStats,
    shutdown: Arc<AtomicBool>,
    logger: Logger,
}

impl Runtime {
    /// Builds the runtime from the full `OmniMeshConfig` produced by the config loader.
    ///
    /// Bug 2 fix: builds a real `TransportConfig` from TOML-parsed addresses instead of
    /// using the silent `TransportConfig::default()` which ignored `omni-mesh.toml`.
    /// Bug 8 fix: uses `new_with_did` so MockTransport in Development mode is bound to
    /// this node's actual DID, allowing the daemon to receive messages.
    pub fn initialize(cfg: OmniMeshConfig) -> Result<Self, String> {
        let self_did = cfg.node_id;

        // Bug 2 fix: build TransportConfig from the TOML-parsed addresses
        let transport_config = TransportConfig {
            tcp_listen_addr: cfg.tcp_listen_addr,
            tcp_connect_addr: cfg.tcp_connect_addr,
            quic_listen_addr: cfg.quic_listen_addr,
            max_read_buffer: cfg.max_read_buffer,
            gossip_bind_addr: cfg.gossip_bind_addr,
            gossip_interval_ms: cfg.gossip_interval_ms,
            multicast_addr: cfg.multicast_addr,
            connection_timeout_secs: 30,
        };

        // Bug 8 fix: bind the mock transport to the real node DID so the daemon
        // can receive messages in Development mode (previously used zero-DID).
        let transport = TransportLayer::with_config_and_routing_did(
            &cfg.mode,
            transport_config,
            std::sync::Arc::new(crate::runtime::RoutingTable::new()),
            self_did,
        )?;

        let security = SecurityLayer::new(&cfg.mode, None);
        let storage = StorageLayer::new(&cfg.mode);
        let delivery = DeliveryLayer::new(&cfg.mode);
        let shutdown = Arc::new(AtomicBool::new(false));
        let logger = Logger::new(LogLevel::Info, true);

        transport.initialize()?;
        security.initialize()?;
        storage.initialize()?;
        delivery.initialize()?;

        Ok(Self {
            self_did,
            transport,
            security,
            storage,
            delivery,
            stats: RuntimeStats::new(),
            shutdown,
            logger,
        })
    }

    pub fn run(cfg: OmniMeshConfig) -> Result<(), String> {
        let mut runtime = Self::initialize(cfg)?;
        runtime.start()?;
        Ok(())
    }

    /// Returns a clone of the shutdown flag for external signal handling.
    pub fn shutdown_handle(&self) -> Arc<AtomicBool> {
        self.shutdown.clone()
    }

    /// Start the runtime daemon loop.
    ///
    /// In daemon mode, this continuously polls the transport for incoming
    /// envelopes. It checks if the envelope is for this node (deliver locally)
    /// or if it should be routed to another node in the mesh network.
    ///
    /// The loop exits gracefully when the shutdown flag is set (e.g. via Ctrl+C).
    fn start(&mut self) -> Result<(), String> {
        // Register Ctrl+C handler for graceful shutdown
        let shutdown_flag = self.shutdown.clone();
        let _ = ctrlc_shutdown(shutdown_flag.clone());

        self.logger.log(
            LogEntry::new(LogLevel::Info, "OMNI-MESH V7 Runtime Daemon starting")
                .with_operation("bootstrap"),
        );

        println!("╔══════════════════════════════════════════════╗");
        println!("║        OMNI-MESH V7 Runtime Daemon          ║");
        println!("╠══════════════════════════════════════════════╣");
        println!(
            "║  Node DID  : {:<31} ║",
            hex::encode(&self.self_did.0[..4])
        );
        println!("║  Transport : {:<31} ║", self.transport.kind());
        println!("║  Security  : {:<31} ║", self.security.kind());
        println!("║  Storage   : {:<31} ║", self.storage.kind());
        println!("║  Delivery  : {:<31} ║", self.delivery.kind());
        println!("╚══════════════════════════════════════════════╝");
        println!();
        println!("Daemon loop started. Press Ctrl+C to shut down gracefully.");

        while !self.shutdown.load(Ordering::Relaxed) {
            match self.transport.receive() {
                Some(envelope) => {
                    self.stats.record_received();

                    // Step 1: Verify the envelope signature
                    match self.security.verify(&envelope) {
                        Ok(_) => {
                            // Step 2: Routing decision
                            if envelope.header.recipient_did == self.self_did {
                                // Delivered to self
                                if let Err(e) = self.storage.store(envelope) {
                                    self.logger.log(
                                        LogEntry::new(LogLevel::Error, "Storage failed")
                                            .with_operation("store")
                                            .with_error(e),
                                    );
                                    continue;
                                }

                                if let Some(last) = self.storage.last_stored() {
                                    if let Err(e) = self.delivery.deliver(last) {
                                        self.logger.log(
                                            LogEntry::new(LogLevel::Error, "Delivery failed")
                                                .with_operation("deliver")
                                                .with_error(e),
                                        );
                                    } else {
                                        self.stats.record_delivered();
                                        let snap = self.stats.snapshot();
                                        self.logger.log(
                                            LogEntry::new(
                                                LogLevel::Info,
                                                "Envelope delivered locally",
                                            )
                                            .with_operation("deliver")
                                            .with_message_id(hex::encode(
                                                envelope.header.message_id.0,
                                            )),
                                        );
                                        let _ = snap; // used for structured log above
                                    }
                                }
                            } else {
                                // Forwarding to neighbor
                                self.logger.log(
                                    LogEntry::new(LogLevel::Info, "Forwarding message")
                                        .with_operation("route")
                                        .with_recipient(hex::encode(
                                            &envelope.header.recipient_did.0[..4],
                                        )),
                                );
                                if let Err(e) = self.transport.send(&envelope) {
                                    self.logger.log(
                                        LogEntry::new(LogLevel::Error, "Forward failed")
                                            .with_operation("route")
                                            .with_error(e),
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            self.stats.record_signature_fail();
                            self.logger.log(
                                LogEntry::new(LogLevel::Warn, "Envelope rejected")
                                    .with_operation("verify")
                                    .with_error(e),
                            );
                        }
                    }
                }
                None => {
                    // No envelope available right now. Yield the thread briefly.
                    std::thread::sleep(std::time::Duration::from_micros(500));
                }
            }
        }

        // Graceful shutdown
        let snap = self.stats.snapshot();
        self.logger.log(
            LogEntry::new(LogLevel::Info, "Daemon shutting down gracefully")
                .with_operation("shutdown"),
        );
        println!(
            "\nShutdown complete. Stats: received={}, delivered={}, rejected={}",
            snap.total_messages_received,
            snap.total_messages_delivered,
            snap.total_dropped_signature_fail
        );

        Ok(())
    }
}

pub fn run(cfg: OmniMeshConfig) -> Result<(), String> {
    Runtime::run(cfg)
}

/// Sets up a Ctrl+C handler that flips the shutdown flag.
/// Returns Ok(()) if the handler was installed, Err if it couldn't be.
fn ctrlc_shutdown(flag: Arc<AtomicBool>) -> Result<(), String> {
    ctrlc::set_handler(move || {
        eprintln!("\nReceived shutdown signal (Ctrl+C). Shutting down...");
        flag.store(true, Ordering::SeqCst);
    })
    .map_err(|e| format!("Failed to set Ctrl+C handler: {}", e))
}
