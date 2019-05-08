use std::collections::{btree_map, BTreeMap};
use std::ptr;

use crate::level_search::{LNode, LevelSearchable};

type LinkedBTree<K, V> = LNode<K, BTreeRange<K, V>>;

#[derive(Default, Debug)]
pub struct BTreeRange<K: LevelSearchable<BTreeRange<K, V>>, V> {
    btree: BTreeMap<K, V>,
    pub(super) max: K,
    pub(super) min: K,
}

impl<K: LevelSearchable<BTreeRange<K, V>>, V> BTreeRange<K, V> {
    pub(super) fn new(key: K) -> BTreeRange<K, V> {
        BTreeRange {
            max: key,
            min: key,
            btree: BTreeMap::new(),
        }
    }

    pub(super) fn insert(&mut self, key: K, value: V) -> Option<V> {
        if key > self.max {
            self.max = key;
        }
        if key < self.min {
            self.min = key;
        }

        self.btree.insert(key, value)
    }

    pub(super) fn get(&self, key: K) -> Option<&V> {
        self.btree.get(&key)
    }

    pub(super) fn remove(&mut self, key: K, default: K) -> Option<V> {
        let output = self.btree.remove(&key);
        if key == self.max {
            self.max = *self.btree.keys().next_back().unwrap_or(&default);
        }
        if key == self.min {
            self.min = *self.btree.keys().next().unwrap_or(&default);
        }

        output
    }

    pub(super) fn keys(&self) -> btree_map::Keys<'_, K, V> {
        self.btree.keys()
    }

    pub(super) fn iter(&self) -> btree_map::Iter<'_, K, V> {
        self.btree.iter()
    }

    pub(super) fn within_range(&self, key: K) -> bool {
        self.min <= key && key <= self.max
    }

    pub(super) fn contains_key(&self, key: K) -> bool {
        self.within_range(key) && self.btree.contains_key(&key)
    }

    pub(super) fn predecessor(&self, key: K) -> Option<(K, &V)> {
        if key < self.min {
            return None;
        }
        self.btree.range(..=key).next_back().map(|(k, v)| (*k, v))
    }

    pub(super) fn successor(&self, key: K) -> Option<(K, &V)> {
        if key > self.max {
            return None;
        }
        self.btree.range(key..).next().map(|(k, v)| (*k, v))
    }
}

impl<K: LevelSearchable<BTreeRange<K, V>>, V> LinkedBTree<K, V> {
    pub(super) fn is_full(&self) -> bool {
        self.value.btree.len() == K::LEN * 2
    }

    pub(super) fn is_small(&self) -> bool {
        self.value.btree.len() <= K::LEN / 2
            && !(self.next.is_null() && self.prev.is_null())
    }

    fn split_point(&self) -> (K, K) {
        // TODO: this split should be done in O(lg n) time, not O(n)
        // time like we're doing here
        //
        // Ideally, BTreeMap would just expose the interface needed to
        // split it into two halves directly...
        debug_assert!(self.value.btree.len() >= K::LEN * 2);
        let mut iter = self.value.btree.keys().skip(K::LEN);
        let low = *iter.next().unwrap();
        let high = *iter.next().unwrap();

        debug_assert!(low < high);
        (low, high)
    }

    pub(super) fn split(&mut self) -> Box<LinkedBTree<K, V>> {
        debug_assert!(self.is_full());
        let (low, high) = self.split_point();

        if self.key <= low {
            let max = std::mem::replace(&mut self.value.max, low);
            return Box::new(LNode {
                key: high,
                value: BTreeRange {
                    btree: self.value.btree.split_off(&high),
                    min: high,
                    max: max,
                },
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            });
        } else {
            debug_assert!(self.key >= high);
            let high_points = self.value.btree.split_off(&high);
            let min = std::mem::replace(&mut self.value.min, high);
            return Box::new(LNode {
                key: low,
                value: BTreeRange {
                    btree: std::mem::replace(
                        &mut self.value.btree,
                        high_points,
                    ),
                    max: low,
                    min: min,
                },
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            });
        }
    }

    pub(super) fn remove(&mut self) -> &mut LinkedBTree<K, V> {
        debug_assert!(self.is_small());

        if let Some(next) = unsafe { self.next.as_mut() } {
            next.value.min = self.value.min;
            next.value.btree.append(&mut self.value.btree);
            return next;
        } else if let Some(prev) = unsafe { self.prev.as_mut() } {
            prev.value.max = self.value.max;
            prev.value.btree.append(&mut self.value.btree);
            return prev;
        }
        unreachable!();
    }
}
