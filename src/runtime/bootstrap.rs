use crate::config::OmnimeshMode;
use crate::runtime::delivery::DeliveryLayer;
use crate::runtime::security::SecurityLayer;
use crate::runtime::storage::StorageLayer;
use crate::runtime::transport::TransportLayer;
use crate::runtime::RuntimeLayer;
use crate::runtime::RuntimeStats;

#[derive(Debug)]
pub struct Runtime {
    transport: TransportLayer,
    security: SecurityLayer,
    storage: StorageLayer,
    delivery: DeliveryLayer,
    stats: RuntimeStats,
}

impl Runtime {
    pub fn initialize(mode: OmnimeshMode) -> Result<Self, String> {
        let transport = TransportLayer::new(&mode)?;
        let security = SecurityLayer::new(&mode, None);
        let storage = StorageLayer::new(&mode);
        let delivery = DeliveryLayer::new(&mode);

        transport.initialize()?;
        security.initialize()?;
        storage.initialize()?;
        delivery.initialize()?;

        Ok(Self {
            transport,
            security,
            storage,
            delivery,
            stats: RuntimeStats::new(),
        })
    }

    pub fn run(mode: OmnimeshMode) -> Result<(), String> {
        let mut runtime = Self::initialize(mode)?;
        runtime.start()?;
        Ok(())
    }

    /// Start the runtime daemon loop.
    ///
    /// In daemon mode, this continuously polls the transport for incoming
    /// envelopes, verifies them, stores them, and delivers them to the
    /// application layer. It runs indefinitely until the process is killed.
    fn start(&mut self) -> Result<(), String> {
        println!("╔══════════════════════════════════════════════╗");
        println!("║        OMNI-MESH V7 Runtime Daemon          ║");
        println!("╠══════════════════════════════════════════════╣");
        println!("║  Transport : {:<31} ║", self.transport.kind());
        println!("║  Security  : {:<31} ║", self.security.kind());
        println!("║  Storage   : {:<31} ║", self.storage.kind());
        println!("║  Delivery  : {:<31} ║", self.delivery.kind());
        println!("╚══════════════════════════════════════════════╝");
        println!();
        println!("Daemon loop started. Polling for envelopes...");

        loop {
            match self.transport.receive() {
                Some(envelope) => {
                    self.stats.record_received();

                    // Pipeline: Verify → Store → Deliver
                    match self.security.verify(&envelope) {
                        Ok(_) => {
                            if let Err(e) = self.storage.store(envelope.clone()) {
                                eprintln!("  [STORE ERR] {}", e);
                                continue;
                            }

                            if let Some(last) = self.storage.last_stored() {
                                if let Err(e) = self.delivery.deliver(last) {
                                    eprintln!("  [DELIVER ERR] {}", e);
                                } else {
                                    self.stats.record_delivered();
                                    let snap = self.stats.snapshot();
                                    println!(
                                        "  [OK] Envelope delivered | recv={} delivered={}",
                                        snap.total_messages_received,
                                        snap.total_messages_delivered,
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            self.stats.record_signature_fail();
                            eprintln!("  [REJECT] {}", e);
                        }
                    }
                }
                None => {
                    // No envelope available right now. Yield the thread briefly.
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
            }
        }
    }
}

pub fn run(mode: OmnimeshMode) -> Result<(), String> {
    Runtime::run(mode)
}

