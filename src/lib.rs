#![cfg_attr(not(feature = "std"), no_std)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::len_zero)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::needless_borrows_for_generic_args)]
#![allow(clippy::manual_flatten)]
#![allow(clippy::redundant_pattern_matching)]
#![allow(clippy::match_like_matches_macro)]

extern crate alloc;

pub mod buffer;
pub mod config;
pub mod envelope;
pub mod payload;

#[cfg(feature = "std")]
pub mod runtime;

#[cfg(feature = "std")]
pub mod client;

pub use buffer::{PayloadError, PayloadStorage, SafetyBufferPool};
pub use config::{CryptoMode, OmnimeshMode, PersistenceMode, WcetMode};
pub use envelope::{Did, EnvelopeHeader, MessageId, PayloadType, Priority, SignedEnvelope};

#[cfg(feature = "std")]
pub use runtime::run;

#[cfg(feature = "std")]
pub use client::{ClientConfig, ClientHealth, OmnimeshClient, ReceivedMessage};
