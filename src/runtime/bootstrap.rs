use crate::config::OmnimeshMode;
use crate::runtime::delivery::DeliveryLayer;
use crate::runtime::security::SecurityLayer;
use crate::runtime::storage::StorageLayer;
use crate::runtime::transport::TransportLayer;

#[derive(Debug)]
pub struct Runtime {
    transport: TransportLayer,
    security: SecurityLayer,
    storage: StorageLayer,
    delivery: DeliveryLayer,
}

impl Runtime {
    pub fn initialize(mode: OmnimeshMode) -> Result<Self, String> {
        let transport = TransportLayer::new(&mode)?;
        let security = SecurityLayer::new(&mode);
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
        })
    }

    pub fn run(mode: OmnimeshMode) -> Result<(), String> {
        let mut runtime = Self::initialize(mode)?;
        runtime.start()?;
        Ok(())
    }

    fn start(&mut self) -> Result<(), String> {
        println!("OMNI-MESH runtime started with layers:");
        println!("  - transport: {}", self.transport.kind());
        println!("  - security: {}", self.security.kind());
        println!("  - storage: {}", self.storage.kind());
        println!("  - delivery: {}", self.delivery.kind());

        if let Some(envelope) = self.transport.receive() {
            self.security.verify(&envelope)?;
            self.storage.store(envelope)?;
            let stored_envelope = self.storage.stored_count();
            println!("Storage contains {} envelope(s)", stored_envelope);
            if let Some(last_envelope) = self.storage.last_stored() {
                self.delivery.deliver(last_envelope)?;
            }
        }

        Ok(())
    }
}

pub fn run(mode: OmnimeshMode) -> Result<(), String> {
    Runtime::run(mode)
}
