use crate::config::OmnimeshMode;
use crate::config::modes::layer_kinds;
use crate::envelope::{Did, SignedEnvelope};
use crate::runtime::RuntimeLayer;
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};

pub trait HardwareSigner: Send + Sync + std::fmt::Debug {
    fn sign(&self, hash: &[u8; 32]) -> Result<[u8; 64], String>;
    fn public_key(&self) -> Did;
}

#[derive(Debug)]
pub struct DefaultSoftwareSigner {
    keypair: SigningKey,
}

impl DefaultSoftwareSigner {
    pub fn new(keypair: SigningKey) -> Self {
        Self { keypair }
    }
}

impl HardwareSigner for DefaultSoftwareSigner {
    fn sign(&self, hash: &[u8; 32]) -> Result<[u8; 64], String> {
        Ok(self.keypair.sign(hash).to_bytes())
    }

    fn public_key(&self) -> Did {
        Did(self.keypair.verifying_key().to_bytes())
    }
}

#[derive(Debug)]
pub struct SecurityLayer {
    kind: &'static str,
    crypto_required: bool,
    _signer: Option<alloc::boxed::Box<dyn HardwareSigner>>,
}

impl SecurityLayer {
    pub fn new(mode: &OmnimeshMode, signer: Option<alloc::boxed::Box<dyn HardwareSigner>>) -> Self {
        let kind = match mode {
            OmnimeshMode::Development(_) => layer_kinds::OPTIONAL_SECURITY,
            OmnimeshMode::Lightweight(_) => layer_kinds::MINIMAL_SECURITY,
            OmnimeshMode::Production(_) => layer_kinds::STANDARD_SECURITY,
        };

        let crypto_required = match mode {
            OmnimeshMode::Production(cfg) => cfg.crypto_enabled,
            OmnimeshMode::Development(cfg) => {
                cfg.crypto_signatures == crate::config::CryptoMode::Required
            }
            OmnimeshMode::Lightweight(cfg) => cfg.crypto_enabled,
        };

        SecurityLayer {
            kind,
            crypto_required,
            _signer: signer,
        }
    }

    pub fn verify<const N: usize>(&self, envelope: &SignedEnvelope<N>) -> Result<(), String> {
        let unsigned = envelope.signature.iter().all(|&b| b == 0);
        if unsigned {
            if self.crypto_required {
                return Err(format!(
                    "security failure: unsigned envelope in {}",
                    self.kind
                ));
            } else {
                // Bug 6 fix: was println! on every message — now only in debug builds
                #[cfg(debug_assertions)]
                eprintln!("[DEBUG] Accepted unsigned envelope in {}", self.kind);
                return Ok(());
            }
        }

        // The DID is assumed to be the 32-byte Ed25519 public key
        let public_key = VerifyingKey::from_bytes(&envelope.header.sender_did.0)
            .map_err(|e| format!("Invalid sender DID (not a valid Ed25519 public key): {}", e))?;

        match envelope.verify_signature(&public_key) {
            Ok(_) => {
                // Bug 6 fix: was println! on every message — now only in debug builds
                #[cfg(debug_assertions)]
                eprintln!("[DEBUG] Verified envelope signature in {}", self.kind);
                Ok(())
            }
            Err(e) => Err(format!("Cryptographic verification failed: {}", e)),
        }
    }
}

impl RuntimeLayer for SecurityLayer {
    fn kind(&self) -> &'static str {
        self.kind
    }

    fn layer_name(&self) -> &'static str {
        "security"
    }
}
