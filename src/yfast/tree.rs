#![allow(dead_code)]
use std::collections::BTreeMap;
use std::ptr;

use crate::level_search::{LNode, LevelSearchable};
use super::LinkedBTree;

impl<K: LevelSearchable<BTreeMap<K, V>>, V> LinkedBTree<K, V> {
    pub(super) fn is_full(&self) -> bool {
        self.value.len() == K::LEN * 2
    }

    fn split_point(&self) -> (K, K) {
        // TODO: this split should be done in O(lg n) time, not O(n)
        // time like we're doing here
        //
        // Ideally, BTreeMap would just expose the interface needed to
        // split it into two halves directly...
        debug_assert!(self.value.len() >= K::LEN * 2);
        let mut iter = self.value.keys().skip(K::LEN);
        let low = *iter.next().unwrap();
        let high = *iter.next().unwrap();

        debug_assert!(low < high);
        (low, high)
    }

    pub(super) fn split(&mut self) -> Box<LinkedBTree<K, V>> {
        debug_assert!(self.is_full());
        let (low, high) = self.split_point();

        if self.key <= low {
            return Box::new(LNode {
                key: high,
                value: self.value.split_off(&high),
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            });
        } else {
            debug_assert!(self.key >= high);
            let high_points = self.value.split_off(&high);
            return Box::new(LNode {
                key: low,
                value: std::mem::replace(&mut self.value, high_points),
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            });
        }
    }
}

/*
#[derive(Debug, PartialEq, Eq)]
pub(super) struct LinkedBTree<K: Ord + Copy, V> {
    representative: K,
    values: BTreeMap<K, V>,

    prev: *mut LinkedBTree<K, V>,
    next: *mut LinkedBTree<K, V>,
}

impl<K: Copy + Ord, V> LinkedBTree<K, V> {
    fn is_full(&self) -> bool {
        self.values.len() == CAPACITY
    }

    fn median(&self) -> (K, K) {
        // TODO: this split should be done in O(lg n) time, not O(n)
        // time like we're doing here
        //
        // Ideally, BTreeMap would just expose the interface needed to
        // split it into two halves directly...
        debug_assert!(self.values.len() >= CAPACITY);
        let mut iter = self.values.keys().skip(CAPACITY / 2);
        let low = *iter.next().unwrap();
        let high = *iter.next().unwrap();

        debug_assert!(low < high);
        (low, high)
    }

    fn predecessor(&self, key: K) -> Option<(K, &V)> {
        self.values.range(..=key).rev().next().map(|(k, v)| (*k, v))
    }

    fn successor(&self, key: K) -> Option<(K, &V)> {
        self.values.range(key..).next().map(|(k, v)| (*k, v))
    }

    fn insert(&mut self, key: K, value: V) -> Option<V> {
        if self.is_full() {
            let (low, high) = self.median();
            self.representative = low;

            let mut next = Box::new(LinkedBTree {
                representative: high,
                values: self.values.split_off(&high),
                prev: self,
                next: self.next,
            });

            let output = if key >= high {
                next.values.insert(key, value)
            } else {
                self.values.insert(key, value)
            };

            let raw = Box::into_raw(next);
            if let Some(next) = unsafe { self.next.as_mut() } {
                next.prev = raw;
            }
            debug_assert_ne!(raw, self.next);
            self.next = raw;

            return output;
        }
        self.values.insert(key, value)
    }

    fn remove(&mut self, key: K) -> Option<V> {
        let output = self.values.remove(&key);

        if self.len() < MIN_SIZE {
            if let Some(next) = unsafe { self.next.as_mut() } {
                debug_assert!(next.len() >= MIN_SIZE);
                debug_assert!(self.len() == MIN_SIZE - 1);

                self.values.append(&mut next.values);
                if self.len() < COMBINE_THRESHOLD {
                    unsafe {
                        drop(Box::from_raw(ptr::replace(
                            &mut self.next,
                            ptr::null_mut(),
                        )));
                    }
                } else {
                    let (low, high) = self.median();
                    self.representative = low;
                    next.representative = high;
                    next.values = self.values.split_off(&high);
                }
            } else if let Some(prev) = unsafe { self.prev.as_mut() } {
                debug_assert!(prev.len() >= MIN_SIZE);
                debug_assert!(self.len() == MIN_SIZE - 1);

                if self.len() + prev.len() < COMBINE_THRESHOLD {
                    self.values.append(&mut prev.values);
                    unsafe {
                        drop(Box::from_raw(ptr::replace(
                            &mut self.prev,
                            ptr::null_mut(),
                        )));
                    }
                } else {
                    let (low, high) = self.median();
                    prev.values.append(&mut self.values);
                    self.representative = high;
                    prev.representative = low;
                    self.values = prev.values.split_off(&high);
                }
            }
        }

        output
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_insertions() {
        let mut map = LinkedBTree::new(0u32, 0u16);

        for i in (0..1000).rev() {
            map.insert(i, i as u16);
        }

        // We always split upwards
        assert!(map.prev.is_null());

        let mut current = &map;
        for i in 0..1000 {
            match current.values.get(&i) {
                Some(value) => assert_eq!(*value, i as u16),
                None => {
                    let prev = current;
                    current = unsafe { current.next.as_ref() }.unwrap();

                    // Linked List Integrity checks
                    assert_ne!(prev, current);
                    assert_eq!(unsafe { current.prev.as_ref() }.unwrap(), prev);

                    assert!(prev.representative < current.representative);
                }
            }
        }

        // We should be at the end of the linked list
        assert!(current.next.is_null());
    }
}
*/
