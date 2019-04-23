#![allow(dead_code)]
use std::collections::BTreeMap;
use std::ptr;

const CAPACITY: usize = 96;
const COMBINE_THRESHOLD: usize = 60;
const MIN_SIZE: usize = 24;

#[derive(Debug, Eq, PartialEq)]
pub(super) struct LinkedBTree<T> {
    representative: u32,
    values: BTreeMap<u32, T>,

    prev: *mut LinkedBTree<T>,
    next: *mut LinkedBTree<T>,
}

impl<T> LinkedBTree<T> {
    fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    fn len(&self) -> usize {
        self.values.len()
    }

    fn is_full(&self) -> bool {
        self.values.len() == CAPACITY
    }

    fn median(&self) -> (u32, u32) {
        // TODO: this split should be done in O(lg n) time, not O(n)
        // time like we're doing here
        //
        // Ideally, BTreeMap would just expose the interface needed to
        // split it into two halves directly...
        debug_assert!(self.values.len() > CAPACITY);
        let mut iter = self.values.keys().skip(CAPACITY / 2);
        let low = *iter.next().unwrap();
        let high = *iter.next().unwrap();
        (low, high)
    }

    fn predecessor(&self, key: u32) -> Option<(u32, &T)> {
        self.values.range(..=key).rev().next().map(|(k, v)| (*k, v))
    }

    fn successor(&self, key: u32) -> Option<(u32, &T)> {
        self.values.range(key..).next().map(|(k, v)| (*k, v))
    }

    fn insert(&mut self, key: u32, value: T) -> Option<T> {
        if self.is_full() {
            let (low, high) = self.median();
            self.representative = low;

            let mut next = Box::new(LinkedBTree {
                representative: high,
                values: self.values.split_off(&high),
                prev: self,
                next: self.next,
            });
            let ptr: &mut LinkedBTree<T> = &mut next;
            self.next = ptr;

            if self.representative < key {
                return next.values.insert(key, value);
            }
        }
        self.values.insert(key, value)
    }

    fn remove(&mut self, key: u32) -> Option<T> {
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
mod test {}
