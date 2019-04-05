use std::collections::{hash_map::Entry, HashMap};
use std::ptr;

struct XFastTrie<T> {
    lss: LevelSearch<T>,
    map: HashMap<u64, *mut LNode<u64, T>>,
    values: *mut LNode<u64, T>,
}

fn lower_bits(key: u64, n: usize) -> u64 {
    // Extract lower bits from key
    key & ((1 << n) - 1)
}

impl<T> XFastTrie<T> {
    fn new() -> XFastTrie<T> {
        let mut lss: LevelSearch<T> = unsafe { std::mem::uninitialized() };
        for l in lss.iter_mut() {
            unsafe {
                ptr::write(l, HashMap::new());
            }
        }
        XFastTrie {
            values: ptr::null_mut(),
            map: HashMap::new(),
            lss,
        }
    }

    fn insert(&mut self, key: u64, value: T) -> Option<T> {
        match self.map.entry(key) {
            Entry::Occupied(mut o) => {
                let node = unsafe { o.get_mut().as_mut().unwrap() };
                Some(std::mem::replace(&mut node.value, value))
            }
            Entry::Vacant(mentry) => {
                let node = Box::into_raw(Box::new(LNode {
                    key,
                    value,
                    prev: ptr::null_mut(),
                    next: ptr::null_mut(),
                }));
                for (i, l) in self.lss.iter_mut().enumerate().rev() {
                    let next_bit = key & (1 << (i + 1));
                    let child = if next_bit == 0 {
                        Descendant::Zero { max: node }
                    } else {
                        Descendant::One { min: node }
                    };

                    match l.entry(lower_bits(key, i)) {
                        Entry::Vacant(v) => {
                            v.insert(child);
                        }

                        Entry::Occupied(mut o) => match (o.get_mut(), child) {
                            (Descendant::Both, _) => {
                                mentry.insert(node);
                                return None;
                            }
                            (
                                Descendant::Zero { .. },
                                Descendant::One { .. },
                            ) => {
                                o.insert(Descendant::Both);
                                mentry.insert(node);
                                return None;
                            }
                            (
                                Descendant::One { .. },
                                Descendant::Zero { .. },
                            ) => {
                                o.insert(Descendant::Both);
                                mentry.insert(node);
                                return None;
                            }
                            (
                                Descendant::Zero { max: m1 },
                                Descendant::Zero { max: m2 },
                            ) => {
                                let m1 = unsafe { m1.as_mut().unwrap() };
                                let m2 = unsafe { m2.as_mut().unwrap() };

                                if m2.key > m1.key {
                                    o.insert(Descendant::Zero { max: m2 });
                                }
                            }
                            (
                                Descendant::One { min: m1 },
                                Descendant::One { min: m2 },
                            ) => {
                                let m1 = unsafe { m1.as_mut().unwrap() };
                                let m2 = unsafe { m2.as_mut().unwrap() };

                                if m2.key < m1.key {
                                    o.insert(Descendant::One { min: m2 });
                                }
                            }
                            (_, _) => unreachable!(),
                        },
                    }
                }

                mentry.insert(node);
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
            let node = unsafe { node.as_mut().unwrap() };
            debug_assert!(node.key == key);
            return Some(&node.value);
        }

        // Special-case: no suffix exists
        let mut bit = key & 0b1;
        let mut level = match self.lss[0].get(&bit) {
            None => {
                bit ^= 0b1;
                self.lss[0].get(&bit).unwrap()
            }
            Some(p) => p,
        };

        // Otherwise, do binary search to find longest existing suffix
        let mut left = 1;
        let mut right = 64;

        while left <= right {
            let mid = (left + right) / 2;
            match self.lss[mid].get(&lower_bits(key, mid)) {
                None => right = mid - 1,
                Some(m) => {
                    level = m;
                    left = mid + 1;
                }
            }
        }

        match level {
            Descendant::Both => unreachable!(),
            Descendant::One { min } => {
                let prev =
                    unsafe { min.as_mut().unwrap().prev.as_ref().unwrap() };
                Some(&prev.value)
            }
            Descendant::Zero { max } => {
                let right = unsafe { max.as_mut().unwrap() };
                Some(&right.value)
            }
        }
    }
}

type LevelSearch<T> = [HashMap<u64, Descendant<T>>; 63];

#[derive(Debug, Eq, PartialEq)]
enum Descendant<T> {
    Both,
    Zero { max: *mut LNode<u64, T> },
    One { min: *mut LNode<u64, T> },
}

struct LNode<K: Eq + PartialEq, V> {
    key: K,
    value: V,

    prev: *mut LNode<K, V>,
    next: *mut LNode<K, V>,
}

#[cfg(test)]
mod test {}
