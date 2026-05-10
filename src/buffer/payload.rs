#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PayloadStorage<const N: usize> {
    len: usize,
    data: [u8; N],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadError {
    Overflow,
}

impl<const N: usize> Default for PayloadStorage<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> PayloadStorage<N> {
    pub const fn new() -> Self {
        Self {
            len: 0,
            data: [0u8; N],
        }
    }

    pub fn capacity(&self) -> usize {
        N
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data[..self.len]
    }

    pub fn clear(&mut self) {
        self.len = 0;
    }

    pub fn push_bytes(&mut self, bytes: &[u8]) -> Result<(), PayloadError> {
        if self.len + bytes.len() > N {
            return Err(PayloadError::Overflow);
        }

        self.data[self.len..self.len + bytes.len()].copy_from_slice(bytes);
        self.len += bytes.len();
        Ok(())
    }

    pub fn try_from_slice(slice: &[u8]) -> Result<Self, PayloadError> {
        if slice.len() > N {
            return Err(PayloadError::Overflow);
        }

        let mut storage = Self::new();
        storage.data[..slice.len()].copy_from_slice(slice);
        storage.len = slice.len();
        Ok(storage)
    }
}
