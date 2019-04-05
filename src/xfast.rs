use std::collections::{hash_map::Entry, HashMap};
use std::ptr;

struct XFastTrie<T> {
    lss: LevelSearch<T>,
    map: HashMap<u64, ptr::NonNull<LNode<u64, T>>>,
    values: *mut LNode<u64, T>,
}

fn lower_bits(key: u64, n: usize) -> u64 {
    // Extract lower bits from key
    key & ((1 << n) - 1)
}

impl<T> XFastTrie<T> {
    fn new() -> XFastTrie<T> {
        XFastTrie {
            values: ptr::null_mut(),
            map: HashMap::new(),
            lss: LevelSearch::new(),
        }
    }

    fn insert(&mut self, key: u64, value: T) -> Option<T> {
        match self.map.entry(key) {
            Entry::Occupied(mut o) => {
                let node = unsafe { o.get_mut().as_mut() };
                Some(std::mem::replace(&mut node.value, value))
            }
            Entry::Vacant(v) => {
                let node = self.lss.insert(key, value);
                v.insert(node);
                None
            }
        }
    }

    fn predecessor(&self, key: u64) -> Option<&T> {
        // If we are empty, short-circuit evaluation
        if self.map.is_empty() {
            return None;
        }
        // Special-case: if key is in the map
        if let Some(node) = self.map.get(&key) {
            let node = unsafe { node.as_ref() };
            debug_assert!(node.key == key);
            return Some(&node.value);
        }

        match self.lss.longest_descendant(key) {
            Descendant::Both => unreachable!(),
            Descendant::One { min } => {
                let prev = unsafe { min.as_ref().prev.as_ref() }?;
                Some(&prev.value)
            }
            Descendant::Zero { max } => {
                let max = unsafe { max.as_ref() };
                Some(&max.value)
            }
        }
    }
}

struct LevelSearch<T> {
    maps: [HashMap<u64, Descendant<T>>; 63],
}

impl<T> LevelSearch<T> {
    fn new() -> LevelSearch<T> {
        unsafe {
            let mut lss: LevelSearch<T> = std::mem::uninitialized();
            for map in lss.maps.iter_mut() {
                ptr::write(map, HashMap::new());
            }
            lss
        }
    }

    fn insert(&mut self, key: u64, value: T) -> ptr::NonNull<LNode<u64, T> >{
        let mut node = unsafe {
            ptr::NonNull::new_unchecked(Box::into_raw(Box::new(LNode {
                key,
                value,
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            })))
        };

        for (i, map) in self.maps.iter_mut().enumerate().rev() {
            let next_bit = key & (1 << (i + 1));
            let child = if next_bit == 0 {
                Descendant::Zero { max: node}
            } else {
                Descendant::One { min: node }
            };

            match map.entry(lower_bits(key, i)) {
                Entry::Vacant(v) => {
                    v.insert(child);
                }
                Entry::Occupied(mut o) => match (o.get_mut(), child) {
                    (Descendant::Both, _) => return node,
                    (Descendant::Zero { max: m }, Descendant::One { .. }) => {
                        unsafe {
                            node.as_mut().prev = m.as_ptr();
                            node.as_mut().next = m.as_mut().next;

                            m.as_mut().next = node.as_ptr();
                            if let Some(next) = node.as_mut().next.as_mut() {
                                next.prev = node.as_ptr();
                            }
                        }
                        o.insert(Descendant::Both);
                        return node;
                    }
                    (Descendant::One { min: m }, Descendant::Zero { .. }) => {
                        unsafe {
                            node.as_mut().next = m.as_ptr();
                            node.as_mut().prev = m.as_mut().prev;
                            m.as_mut().prev = node.as_ptr();

                            if let Some(prev) = node.as_mut().prev.as_mut() {
                                prev.next = node.as_ptr();
                            }
                        }
                        o.insert(Descendant::Both);
                        return node;
                    }
                    (
                        Descendant::Zero { max: m1 },
                        Descendant::Zero { max: m2 },
                    ) => {
                        if unsafe { m2.as_ref().key > m1.as_ref().key } {
                            o.insert(Descendant::Zero { max: m2 });
                        }
                    }
                    (
                        Descendant::One { min: m1 },
                        Descendant::One { min: m2 },
                    ) => {
                        if unsafe { m2.as_ref().key < m1.as_ref().key } {
                            o.insert(Descendant::One { min: m2 });
                        }
                    }
                    (_, Descendant::Both) => unreachable!(),
                },
            }
        }
        node
    }

    fn longest_descendant(&self, key: u64) -> &Descendant<T> {
        let mut bit = key & 0b1;
        let mut level = match self.maps[0].get(&bit) {
            None => {
                bit ^= 0b1;
                self.maps[0].get(&bit).unwrap()
            }
            Some(p) => p,
        };

        let mut left = 1;
        let mut right = 63;
        while left <= right {
            let mid = (left + right) / 2;
            match self.maps[mid].get(&lower_bits(key, mid)) {
                None => right = mid - 1,
                Some(m) => {
                    level = m;
                    left = mid + 1;
                }
            }
        }
        level
    }

    fn entry(&mut self, key: u64, level: usize) -> Entry<u64, Descendant<T>> {
        debug_assert!(level < 64);
        self.maps[level].entry(lower_bits(key, level))
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Descendant<T> {
    Both,
    Zero { max: ptr::NonNull<LNode<u64, T>> },
    One { min: ptr::NonNull<LNode<u64, T>> },
}

struct LNode<K: Eq + PartialEq, V> {
    key: K,
    value: V,

    prev: *mut LNode<K, V>,
    next: *mut LNode<K, V>,
}

#[cfg(test)]
mod test {}
