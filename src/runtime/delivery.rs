use crate::config::OmnimeshMode;
use crate::envelope::SignedEnvelope;

#[derive(Debug)]
pub struct DeliveryLayer {
    kind: &'static str,
}

impl DeliveryLayer {
    pub fn new(mode: &OmnimeshMode) -> Self {
        let kind = match mode {
            OmnimeshMode::Development(_) => "best-effort delivery",
            OmnimeshMode::Lightweight(_) => "lightweight delivery",
            OmnimeshMode::Production(_) => "reliable delivery",
            OmnimeshMode::Certified(_) => "certified delivery",
        };

        DeliveryLayer { kind }
    }

    pub fn initialize(&self) -> Result<(), String> {
        println!("Initializing delivery layer: {}", self.kind);
        Ok(())
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn deliver<const N: usize>(&self, envelope: &SignedEnvelope<N>) -> Result<(), String> {
        println!(
            "Delivering payload of {} bytes from {:?} to {:?} through {}",
            envelope.payload.len(),
            envelope.header.sender_did,
            envelope.header.recipient_did,
            self.kind
        );
        Ok(())
    }
}
