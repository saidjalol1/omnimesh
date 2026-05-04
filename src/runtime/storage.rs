use crate::config::OmnimeshMode;
use crate::envelope::SignedEnvelope;

const DEFAULT_PAYLOAD_CAPACITY: usize = 128;

#[derive(Debug)]
pub struct StorageLayer {
    kind: &'static str,
    stored: Vec<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>,
}

impl StorageLayer {
    pub fn new(mode: &OmnimeshMode) -> Self {
        let kind = match mode {
            OmnimeshMode::Development(_) => "development storage",
            OmnimeshMode::Lightweight(_) => "ephemeral storage",
            OmnimeshMode::Production(_) => "persistent storage",
            OmnimeshMode::Certified(_) => "certified storage",
        };

        StorageLayer {
            kind,
            stored: Vec::new(),
        }
    }

    pub fn initialize(&self) -> Result<(), String> {
        println!("Initializing storage layer: {}", self.kind);
        Ok(())
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn store(&mut self, envelope: SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        println!("Storing envelope in {}", self.kind);
        self.stored.push(envelope);
        Ok(())
    }

    pub fn stored_count(&self) -> usize {
        self.stored.len()
    }

    pub fn last_stored(
        &self,
    ) -> Option<&SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>> {
        self.stored.last()
    }
}
