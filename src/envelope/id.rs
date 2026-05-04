#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Did(pub [u8; 32]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MessageId(pub [u8; 16]);

impl Did {
    pub fn new(bytes: [u8; 32]) -> Self {
        Did(bytes)
    }
}

impl MessageId {
    pub fn new(bytes: [u8; 16]) -> Self {
        MessageId(bytes)
    }
}
