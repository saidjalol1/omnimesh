use crate::config::OmnimeshMode;
use crate::envelope::SignedEnvelope;

#[derive(Debug)]
pub struct SecurityLayer {
    kind: &'static str,
}

impl SecurityLayer {
    pub fn new(mode: &OmnimeshMode) -> Self {
        let kind = match mode {
            OmnimeshMode::Development(_) => "optional security",
            OmnimeshMode::Lightweight(_) => "minimal security",
            OmnimeshMode::Production(_) => "standard security",
            OmnimeshMode::Certified(_) => "certified security",
        };

        SecurityLayer { kind }
    }

    pub fn initialize(&self) -> Result<(), String> {
        println!("Initializing security layer: {}", self.kind);
        Ok(())
    }

    pub fn kind(&self) -> &'static str {
        self.kind
    }

    pub fn verify<const N: usize>(
        &self,
        envelope: &SignedEnvelope<N>,
    ) -> Result<(), String> {
        let unsigned = envelope.signature.iter().all(|&b| b == 0);
        if unsigned {
            Err(format!("security failure: unsigned envelope in {}", self.kind))
        } else {
            println!("Verified envelope signature in {}", self.kind);
            Ok(())
        }
    }
}
