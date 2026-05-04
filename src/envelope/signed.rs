use crate::buffer::PayloadStorage;
use super::header::EnvelopeHeader;
use super::wire::{ParseError, RawEnvelopeHeader};

#[derive(Debug, Clone)]
pub struct SignedEnvelope<const N: usize> {
    pub header: EnvelopeHeader,
    pub payload: PayloadStorage<N>,
    pub signature: [u8; 64],
}

impl<const N: usize> SignedEnvelope<N> {
    pub const SIGNATURE_SIZE: usize = 64;

    pub fn new(header: EnvelopeHeader, payload: PayloadStorage<N>, signature: [u8; 64]) -> Self {
        SignedEnvelope {
            header,
            payload,
            signature,
        }
    }

    pub fn serialize(&self) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(RawEnvelopeHeader::SIZE + self.payload.len() + Self::SIGNATURE_SIZE);
        bytes.extend_from_slice(&self.header.to_bytes());
        bytes.extend_from_slice(self.payload.as_slice());
        bytes.extend_from_slice(&self.signature);
        bytes
    }

    pub fn deserialize(buf: &[u8]) -> Result<Self, ParseError> {
        let header = EnvelopeHeader::from_bytes(buf)?;
        let start_of_payload = RawEnvelopeHeader::SIZE;
        let end_of_signature = buf
            .len()
            .checked_sub(Self::SIGNATURE_SIZE)
            .ok_or(ParseError::Truncated)?;

        if end_of_signature < start_of_payload {
            return Err(ParseError::Truncated);
        }

        let payload = PayloadStorage::try_from_slice(&buf[start_of_payload..end_of_signature])
            .map_err(|_| ParseError::PayloadOverflow)?;

        let mut signature = [0u8; 64];
        signature.copy_from_slice(&buf[end_of_signature..]);

        Ok(SignedEnvelope {
            header,
            payload,
            signature,
        })
    }
}
