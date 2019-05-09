use std::ptr;

use crate::level_search::{LNode, LevelSearchable};

type LinkedBTree<K, V> = LNode<K, BTreeRange<K, V>>;

#[derive(Default, Debug)]
pub struct BTreeRange<K: Ord + Copy + std::fmt::Debug, V> {
    pub(super) keys: Vec<K>,
    values: Vec<V>,
}

impl<K: Ord + Copy + std::fmt::Debug, V> BTreeRange<K, V> {
    pub(super) fn new() -> BTreeRange<K, V> {
        BTreeRange {
            keys: Vec::new(),
            values: Vec::new(),
        }
    }

    pub(super) fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.keys.binary_search(&key) {
            Ok(pos) => Some(std::mem::replace(&mut self.values[pos], value)),
            Err(pos) => {
                self.keys.insert(pos, key);
                self.values.insert(pos, value);
                None
            }
        }
    }

    pub(super) fn get(&self, key: K) -> Option<&V> {
        match self.keys.binary_search(&key) {
            Ok(pos) => Some(&self.values[pos]),
            Err(_) => None,
        }
    }

    pub(super) fn remove(&mut self, key: K) -> Option<V> {
        match self.keys.binary_search(&key) {
            Ok(pos) => {
                self.keys.remove(pos);
                Some(self.values.remove(pos))
            }
            Err(_) => None,
        }
    }

    pub(super) fn keys(&self) -> std::iter::Cloned<std::slice::Iter<'_, K>> {
        self.keys.iter().cloned()
    }

    pub(super) fn iter(
        &self,
    ) -> std::iter::Zip<
        std::iter::Cloned<std::slice::Iter<'_, K>>,
        std::slice::Iter<'_, V>,
    > {
        Iterator::zip(self.keys.iter().cloned(), self.values.iter())
    }

    pub(super) fn contains_key(&self, key: K) -> bool {
        self.keys.binary_search(&key).is_ok()
    }

    pub(super) fn predecessor(&self, key: K) -> Option<(K, &V)> {
        match self.keys.binary_search(&key) {
            Ok(pos) => Some((key, &self.values[pos])),
            Err(pos) => {
                if pos > 0 {
                    Some((self.keys[pos - 1], &self.values[pos - 1]))
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn successor(&self, key: K) -> Option<(K, &V)> {
        match self.keys.binary_search(&key) {
            Ok(pos) => Some((key, &self.values[pos])),
            Err(pos) => {
                if pos < self.keys.len() {
                    Some((self.keys[pos], &self.values[pos]))
                } else {
                    None
                }
            }
        }
    }

    pub(super) fn merge(&mut self, other: &mut BTreeRange<K, V>) {
        // TODO: do an actual merge sort
        let len = self.keys.len() + other.keys.len();

        let mut keys =
            std::mem::replace(&mut self.keys, Vec::with_capacity(len));
        let mut values =
            std::mem::replace(&mut self.values, Vec::with_capacity(len));
        let mut iter1 = Iterator::zip(keys.drain(..), values.drain(..));
        let mut iter2 =
            Iterator::zip(other.keys.drain(..), other.values.drain(..));

        let mut p1 = iter1.next();
        let mut p2 = iter2.next();
        loop {
            match (&p1, &p2) {
                (Some(a), Some(b)) => {
                    if a.0 > b.0 {
                        self.keys.push(b.0);
                        self.values.push(p2.take().unwrap().1);
                        p2 = iter2.next();
                    } else {
                        self.keys.push(a.0);
                        self.values.push(p1.take().unwrap().1);
                        p1 = iter1.next();
                    }
                }
                (None, Some(b)) => {
                    self.keys.push(b.0);
                    self.values.push(p2.take().unwrap().1);
                    p2 = iter2.next();
                }
                (Some(a), None) => {
                    self.keys.push(a.0);
                    self.values.push(p1.take().unwrap().1);
                    p1 = iter1.next();
                }
                (None, None) => break,
            }
        }
    }
}

impl<K: LevelSearchable<BTreeRange<K, V>>, V> LinkedBTree<K, V> {
    pub(super) fn is_full(&self) -> bool {
        self.value.keys.len() == K::LEN * 2
    }

    pub(super) fn is_small(&self) -> bool {
        self.value.keys.len() <= K::LEN / 2
            && !(self.next.is_null() && self.prev.is_null())
    }

    pub(super) fn max(&self) -> K {
        self.value.keys.last().cloned().unwrap_or(self.key)
    }

    pub(super) fn min(&self) -> K {
        self.value.keys.first().cloned().unwrap_or(self.key)
    }

    pub(super) fn split(&mut self) -> Box<LinkedBTree<K, V>> {
        debug_assert!(self.is_full());
        let len = self.value.keys.len();

        let median = self.value.keys[len / 2];

        if self.key < median {
            return Box::new(LNode {
                key: median,
                value: BTreeRange {
                    keys: self.value.keys.split_off(len / 2),
                    values: self.value.values.split_off(len / 2),
                },
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            });
        } else {
            let high_keys = self.value.keys.split_off(len / 2);
            let high_values = self.value.values.split_off(len / 2);

            return Box::new(LNode {
                key: median,
                value: BTreeRange {
                    keys: std::mem::replace(&mut self.value.keys, high_keys),
                    values: std::mem::replace(
                        &mut self.value.values,
                        high_values,
                    ),
                },
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            });
        }
    }

    pub(super) fn remove(&mut self) -> &mut LinkedBTree<K, V> {
        debug_assert!(self.is_small());

        if let Some(next) = unsafe { self.next.as_mut() } {
            next.value.merge(&mut self.value);
            return next;
        } else if let Some(prev) = unsafe { self.prev.as_mut() } {
            prev.value.merge(&mut self.value);
            return prev;
        } else {
            unreachable!();
        }
    }

    pub(super) fn within_range(&self, key: K) -> bool {
        *self.value.keys.first().unwrap_or(&self.key) <= key
            && key <= *self.value.keys.last().unwrap_or(&self.key)
    }
}
