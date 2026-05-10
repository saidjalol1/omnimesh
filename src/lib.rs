#![cfg_attr(not(feature = "std"), no_std)]

extern crate alloc;

pub mod buffer;
pub mod config;
pub mod envelope;
pub mod payload;

#[cfg(feature = "std")]
pub mod runtime;

pub use buffer::{PayloadError, PayloadStorage, SafetyBufferPool};
pub use config::{CryptoMode, OmnimeshMode, PersistenceMode, WcetMode};
pub use envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};

#[cfg(feature = "std")]
pub use runtime::run;
