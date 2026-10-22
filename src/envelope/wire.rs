use super::header::{PayloadType, Priority};

#[repr(C, packed)]
pub struct RawEnvelopeHeader {
    pub magic: u32,
    pub version: u32,
    pub message_id: [u8; 16],
    pub sequence: u64,
    pub sender_did: [u8; 32],
    pub recipient_did: [u8; 32],
    pub timestamp_us: u64,
    pub priority: u8,
    pub payload_type: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParseError {
    Truncated,
    InvalidMagic(u32),
    InvalidPriority(u8),
    InvalidPayloadType(u8),
    PayloadOverflow,
}

impl RawEnvelopeHeader {
    pub const MAGIC: u32 = 0x4F4D4E49;
    pub const SIZE: usize = 106;

    pub fn size() -> usize {
        Self::SIZE
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, ParseError> {
        if buf.len() < Self::SIZE {
            return Err(ParseError::Truncated);
        }

        // SAFETY: We use copy_nonoverlapping into a fresh MaybeUninit to avoid
        // unaligned reads from the #[repr(C, packed)] struct. Casting packed
        // struct pointers directly causes SIGBUS on ARMv7 (32-bit Android).
        let mut raw = core::mem::MaybeUninit::<Self>::uninit();
        unsafe {
            core::ptr::copy_nonoverlapping(
                buf.as_ptr(),
                raw.as_mut_ptr() as *mut u8,
                Self::SIZE,
            );
            Ok(raw.assume_init())
        }
    }
}

impl TryFrom<u8> for Priority {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(Priority::Low),
            1 => Ok(Priority::Normal),
            2 => Ok(Priority::High),
            3 => Ok(Priority::Critical),
            invalid => Err(ParseError::InvalidPriority(invalid)),
        }
    }
}

impl TryFrom<u8> for PayloadType {
    type Error = ParseError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(PayloadType::Raw),
            1 => Ok(PayloadType::RobotCommand),
            2 => Ok(PayloadType::ModelWeights),
            3 => Ok(PayloadType::InferenceResult),
            4 => Ok(PayloadType::SensorFusion),
            invalid => Err(ParseError::InvalidPayloadType(invalid)),
        }
    }
}
