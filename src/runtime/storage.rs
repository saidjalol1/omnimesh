use crate::config::OmnimeshMode;
use crate::config::modes::layer_kinds;
use crate::envelope::SignedEnvelope;
use crate::runtime::RuntimeLayer;
use std::sync::Arc;
use sled::Db;

use crate::runtime::transport::interface::DEFAULT_PAYLOAD_CAPACITY;

#[derive(Clone)]
pub struct DtnStore {
    db: Arc<Db>,
}

impl std::fmt::Debug for DtnStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DtnStore").finish()
    }
}

impl DtnStore {
    pub fn new(path: &str) -> Result<Self, String> {
        let db = sled::open(path).map_err(|e| e.to_string())?;
        Ok(DtnStore { db: Arc::new(db) })
    }

    pub fn store_envelope<const N: usize>(&self, envelope: &SignedEnvelope<N>) -> Result<(), String> {
        let key = envelope.header.message_id.0;
        let mut buf = [0u8; 2048];
        let len = envelope.serialize_into(&mut buf).map_err(|e| format!("{:?}", e))?;
        self.db.insert(key, &buf[..len]).map_err(|e| e.to_string())?;
        self.db.flush().map_err(|e| e.to_string())?;
        Ok(())
    }

    /// Stores a message specifically for DTN mule routing
    pub fn store_for_mule<const N: usize>(&self, envelope: &SignedEnvelope<N>) -> Result<(), String> {
        self.store_envelope(envelope)
    }

    /// Retrieves all stored messages destined for the given recipient
    pub fn retrieve_for_recipient<const N: usize>(&self, recipient: &crate::envelope::Did) -> Vec<SignedEnvelope<N>> {
        let mut envelopes = Vec::new();
        let mut keys_to_remove = Vec::new();

        for item in self.db.iter() {
            if let Ok((key, val)) = item {
                if let Ok(env) = SignedEnvelope::<N>::deserialize(&val) {
                    if &env.header.recipient_did == recipient {
                        envelopes.push(env);
                        keys_to_remove.push(key);
                    }
                }
            }
        }

        // Remove retrieved items from DTN store (they have been picked up)
        for key in keys_to_remove {
            let _ = self.db.remove(key);
        }
        let _ = self.db.flush();

        envelopes
    }
}

#[derive(Debug)]
pub struct StorageLayer {
    kind: &'static str,
    stored: Vec<SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>>,
    dtn: Option<DtnStore>,
}

impl StorageLayer {
    pub fn new(mode: &OmnimeshMode) -> Self {
        let mut dtn = None;

        let kind = match mode {
            OmnimeshMode::Development(_) => layer_kinds::DEVELOPMENT_STORAGE,
            OmnimeshMode::Lightweight(_) => layer_kinds::EPHEMERAL_STORAGE,
            OmnimeshMode::Production(cfg) => {
                if cfg.dtn_enabled
                    && let Some(path) = &cfg.dtn_path {
                        dtn = DtnStore::new(path).ok();
                    }
                layer_kinds::PERSISTENT_STORAGE
            }
            OmnimeshMode::Certified(_) => layer_kinds::CERTIFIED_STORAGE,
        };

        StorageLayer {
            kind,
            stored: Vec::new(),
            dtn,
        }
    }

    pub fn store(&mut self, envelope: SignedEnvelope<DEFAULT_PAYLOAD_CAPACITY>) -> Result<(), String> {
        println!("Storing envelope in {}", self.kind);
        
        if let Some(dtn) = &self.dtn {
            dtn.store_envelope(&envelope)?;
            println!("Envelope stored persistently in RocksDB DTN.");
        }
        
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
    
    pub fn dtn_store(&self) -> Option<&DtnStore> {
        self.dtn.as_ref()
    }
}

impl RuntimeLayer for StorageLayer {
    fn kind(&self) -> &'static str {
        self.kind
    }

    fn layer_name(&self) -> &'static str {
        "storage"
    }
}
