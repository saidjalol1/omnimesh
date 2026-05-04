use super::payload::PayloadStorage;

pub struct SafetyBufferPool<const N: usize, const CAPACITY: usize> {
    buffers: [PayloadStorage<N>; CAPACITY],
    free_indices: [usize; CAPACITY],
    free_count: usize,
}

impl<const N: usize, const CAPACITY: usize> SafetyBufferPool<N, CAPACITY> {
    pub fn new() -> Self {
        Self {
            buffers: std::array::from_fn(|_| PayloadStorage::new()),
            free_indices: std::array::from_fn(|i| i),
            free_count: CAPACITY,
        }
    }

    pub fn acquire(&mut self) -> Option<(usize, &mut PayloadStorage<N>)> {
        if self.free_count == 0 {
            return None;
        }

        self.free_count -= 1;
        let index = self.free_indices[self.free_count];
        Some((index, &mut self.buffers[index]))
    }

    pub fn release(&mut self, index: usize) {
        if index >= CAPACITY || self.free_count >= CAPACITY {
            return;
        }

        self.buffers[index].clear();
        self.free_indices[self.free_count] = index;
        self.free_count += 1;
    }

    pub fn utilization(&self) -> f32 {
        let used = CAPACITY - self.free_count;
        used as f32 / CAPACITY as f32
    }
}
