#[derive(Debug, Clone)]
pub struct RingBuffer<T: Copy, const N: usize> {
    data: [Option<T>; N],
    head: usize,
}

impl<T: Copy, const N: usize> Default for RingBuffer<T, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Copy, const N: usize> RingBuffer<T, N> {
    pub const fn new() -> Self {
        Self {
            data: [None; N],
            head: 0,
        }
    }

    pub fn insert(&mut self, item: T) {
        self.data[self.head] = Some(item);
        self.head = (self.head + 1) % N;
    }

    pub fn contains<F>(&self, predicate: F) -> bool
    where
        F: Fn(&T) -> bool,
    {
        for item in self.data.iter() {
            if let Some(val) = item
                && predicate(val) {
                    return true;
                }
        }
        false
    }
}
