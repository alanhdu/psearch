use std::collections::BTreeMap;
use std::ptr;

use super::LinkedBTree;
use crate::level_search::{LNode, LevelSearchable};

impl<K: LevelSearchable<BTreeMap<K, V>>, V> LinkedBTree<K, V> {
    pub(super) fn is_full(&self) -> bool {
        self.value.len() == K::LEN * 2
    }

    pub(super) fn is_small(&self) -> bool {
        self.value.len() <= K::LEN / 2
            && !(self.next.is_null() && self.prev.is_null())
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

    pub(super) fn remove(&mut self) -> &mut LinkedBTree<K, V> {
        debug_assert!(self.is_small());

        if let Some(next) = unsafe { self.next.as_mut() } {
            next.value.append(&mut self.value);
            return next;
        } else if let Some(prev) = unsafe { self.prev.as_mut() } {
            prev.value.append(&mut self.value);
            return prev;
        }
        unreachable!();
    }
}
