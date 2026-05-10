#[derive(Debug, Clone)]
pub struct FixedMap<K: Copy + PartialEq, V: Copy, const N: usize> {
    entries: alloc::boxed::Box<[Option<(K, V)>; N]>,
    count: usize,
}

impl<K: Copy + PartialEq, V: Copy, const N: usize> Default for FixedMap<K, V, N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<K: Copy + PartialEq, V: Copy, const N: usize> FixedMap<K, V, N> {
    pub fn new() -> Self {
        // Use vec! to avoid stack allocation, then convert to Box<[T; N]>
        let mut vec = alloc::vec::Vec::with_capacity(N);
        for _ in 0..N {
            vec.push(None);
        }
        let boxed_slice = vec.into_boxed_slice();
        let entries = match boxed_slice.try_into() {
            Ok(arr) => arr,
            Err(_) => unreachable!("Vec length matches N"),
        };
        
        Self {
            entries,
            count: 0,
        }
    }

    pub fn get(&self, key: &K) -> Option<&V> {
        for entry in self.entries.iter() {
            if let Some((k, v)) = entry
                && k == key {
                    return Some(v);
                }
        }
        None
    }

    pub fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        for entry in self.entries.iter_mut() {
            if let Some((k, v)) = entry
                && k == key {
                    return Some(v);
                }
        }
        None
    }

    pub fn insert(&mut self, key: K, value: V) -> Result<Option<V>, &'static str> {
        // Update if exists
        for entry in self.entries.iter_mut() {
            if let Some((k, v)) = entry
                && *k == key {
                    let old = *v;
                    *v = value;
                    return Ok(Some(old));
                }
        }

        // Find empty slot
        for entry in self.entries.iter_mut() {
            if entry.is_none() {
                *entry = Some((key, value));
                self.count += 1;
                return Ok(None);
            }
        }

        Err("FixedMap is full")
    }

    pub fn remove(&mut self, key: &K) -> Option<V> {
        for entry in self.entries.iter_mut() {
            if let Some((k, v)) = entry
                && k == key {
                    let val = *v;
                    *entry = None;
                    self.count -= 1;
                    return Some(val);
                }
        }
        None
    }

    pub fn len(&self) -> usize {
        self.count
    }

    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    pub fn iter(&self) -> impl Iterator<Item = &(K, V)> {
        self.entries.iter().filter_map(|e| e.as_ref())
    }
}
