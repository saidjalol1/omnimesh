use crate::buffer::PayloadStorage;
use super::header::EnvelopeHeader;
use super::wire::{ParseError, RawEnvelopeHeader};
// Vec no longer needed as serialization writes to slice
use ed25519_dalek::{Signature, Signer, Verifier, VerifyingKey, SigningKey};
use blake3::Hasher;

#[derive(Debug, Clone, Copy)]
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

    pub fn sign(header: EnvelopeHeader, payload: PayloadStorage<N>, keypair: &SigningKey) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&header.to_bytes());
        hasher.update(payload.as_slice());
        let hash = hasher.finalize();
        
        let signature = keypair.sign(hash.as_bytes());
        
        SignedEnvelope {
            header,
            payload,
            signature: signature.to_bytes(),
        }
    }

    pub fn hash_payload(&self) -> [u8; 32] {
        let mut hasher = Hasher::new();
        hasher.update(&self.header.to_bytes());
        hasher.update(self.payload.as_slice());
        hasher.finalize().into()
    }

    pub fn verify_signature(&self, public_key: &VerifyingKey) -> Result<(), ed25519_dalek::SignatureError> {
        let hash = self.hash_payload();
        let signature = Signature::from_bytes(&self.signature);
        public_key.verify(&hash, &signature)
    }

    pub fn serialize_into(&self, buf: &mut [u8]) -> Result<usize, ParseError> {
        let size = RawEnvelopeHeader::SIZE + self.payload.len() + Self::SIGNATURE_SIZE;
        if buf.len() < size {
            return Err(ParseError::PayloadOverflow);
        }
        
        let (header_slice, rest) = buf.split_at_mut(RawEnvelopeHeader::SIZE);
        header_slice.copy_from_slice(&self.header.to_bytes());
        
        let (payload_slice, sig_slice) = rest.split_at_mut(self.payload.len());
        payload_slice.copy_from_slice(self.payload.as_slice());
        
        sig_slice[..Self::SIGNATURE_SIZE].copy_from_slice(&self.signature);
        
        Ok(size)
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
