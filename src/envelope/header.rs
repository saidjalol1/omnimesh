use super::id::{Did, MessageId};
use super::wire::{ParseError, RawEnvelopeHeader};

#[derive(Debug, Clone, Copy)]
pub struct EnvelopeHeader {
    pub version: u32,
    pub message_id: MessageId,
    pub sender_did: Did,
    pub recipient_did: Did,
    pub sequence_number: u64,
    pub timestamp_us: u64,
    pub priority: Priority,
    pub payload_type: PayloadType,
}

impl EnvelopeHeader {
    pub fn new(
        version: u32,
        message_id: MessageId,
        sender_did: Did,
        recipient_did: Did,
        sequence_number: u64,
        timestamp_us: u64,
        priority: Priority,
        payload_type: PayloadType,
    ) -> Self {
        EnvelopeHeader {
            version,
            message_id,
            sender_did,
            recipient_did,
            sequence_number,
            timestamp_us,
            priority,
            payload_type,
        }
    }

    pub fn from_bytes(buf: &[u8]) -> Result<Self, ParseError> {
        let raw = RawEnvelopeHeader::from_bytes(buf)?;

        if raw.magic != RawEnvelopeHeader::MAGIC {
            return Err(ParseError::InvalidMagic(raw.magic));
        }

        Ok(EnvelopeHeader {
            version: u32::from_le(raw.version),
            message_id: MessageId(raw.message_id),
            sender_did: Did(raw.sender_did),
            recipient_did: Did(raw.recipient_did),
            sequence_number: u64::from_le(raw.sequence),
            timestamp_us: u64::from_le(raw.timestamp_us),
            priority: Priority::try_from(raw.priority)?,
            payload_type: PayloadType::try_from(raw.payload_type)?,
        })
    }

    pub fn to_raw(&self) -> RawEnvelopeHeader {
        RawEnvelopeHeader {
            magic: RawEnvelopeHeader::MAGIC,
            version: self.version,
            message_id: self.message_id.0,
            sequence: self.sequence_number.to_le(),
            sender_did: self.sender_did.0,
            recipient_did: self.recipient_did.0,
            timestamp_us: self.timestamp_us.to_le(),
            priority: self.priority.into(),
            payload_type: self.payload_type.into(),
        }
    }

    pub fn to_bytes(&self) -> [u8; RawEnvelopeHeader::SIZE] {
        let raw = self.to_raw();
        let mut bytes = [0u8; RawEnvelopeHeader::SIZE];

        unsafe {
            core::ptr::copy_nonoverlapping(
                &raw as *const RawEnvelopeHeader as *const u8,
                bytes.as_mut_ptr(),
                RawEnvelopeHeader::SIZE,
            );
        }

        bytes
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Priority {
    Low,
    Normal,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    Raw,
    RobotCommand,
    ModelWeights,
    InferenceResult,
    SensorFusion,
}

impl From<Priority> for u8 {
    fn from(priority: Priority) -> Self {
        match priority {
            Priority::Low => 0,
            Priority::Normal => 1,
            Priority::High => 2,
            Priority::Critical => 3,
        }
    }
}

impl From<PayloadType> for u8 {
    fn from(payload_type: PayloadType) -> Self {
        match payload_type {
            PayloadType::Raw => 0,
            PayloadType::RobotCommand => 1,
            PayloadType::ModelWeights => 2,
            PayloadType::InferenceResult => 3,
            PayloadType::SensorFusion => 4,
        }
    }
}
