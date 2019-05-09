use std::collections::hash_map::Entry as HashEntry;

use fnv::FnvHashMap as HashMap;

use super::{BTreeRange, LevelSearchable, LinkedBTree};
use crate::level_search::LNode;

#[derive(Default, Debug)]
pub struct YFastMap<K: LevelSearchable<BTreeRange<K, V>>, V> {
    lss: K::LSS,
    map: HashMap<K, Box<LinkedBTree<K, V>>>,
    len: usize,
}

impl<K: LevelSearchable<BTreeRange<K, V>>, V> YFastMap<K, V> {
    pub fn new() -> YFastMap<K, V> {
        YFastMap {
            lss: K::lss_new(),
            map: HashMap::default(),
            len: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        debug_assert_eq!(self.map.is_empty(), self.len == 0);
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn clear(&mut self) {
        K::lss_clear(&mut self.lss);
        self.map.clear();
    }

    pub fn get(&self, key: K) -> Option<&V> {
        let (byte, desc) = K::lss_longest_descendant(&self.lss, key);
        if let Some(pred) = desc.predecessor(byte) {
            pred.value.get(key).or_else(|| {
                unsafe { pred.next.as_ref() }
                    .and_then(|next| next.value.get(key))
            })
        } else if let Some(succ) = desc.successor(byte) {
            succ.value.get(key).or_else(|| {
                unsafe { succ.prev.as_ref() }
                    .and_then(|prev| prev.value.get(key))
            })
        } else {
            None
        }
    }

    pub fn predecessor(&self, key: K) -> Option<(K, &V)> {
        let (byte, desc) = K::lss_longest_descendant(&self.lss, key);
        if let Some(pred) = desc.predecessor(byte) {
            unsafe { pred.next.as_ref() }
                .and_then(|next| next.value.predecessor(key))
                .or_else(|| pred.value.predecessor(key))
        } else if let Some(succ) = desc.successor(byte) {
            succ.value.predecessor(key).or_else(|| {
                unsafe { succ.prev.as_ref() }
                    .and_then(|prev| prev.value.predecessor(key))
            })
        } else {
            None
        }
    }

    pub fn successor(&self, key: K) -> Option<(K, &V)> {
        let (byte, desc) = K::lss_longest_descendant(&self.lss, key);
        if let Some(pred) = desc.predecessor(byte) {
            pred.value.successor(key).or_else(|| {
                unsafe { pred.next.as_ref() }
                    .and_then(|next| next.value.successor(key))
            })
        } else if let Some(succ) = desc.successor(byte) {
            unsafe { succ.prev.as_ref() }
                .and_then(|prev| prev.value.successor(key))
                .or_else(|| succ.value.successor(key))
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: K) -> bool {
        let (byte, desc) = K::lss_longest_descendant(&self.lss, key);
        if let Some(pred) = desc.predecessor(byte) {
            if pred.within_range(key) {
                pred.value.contains_key(key)
            } else {
                unsafe { pred.next.as_ref() }
                    .map(|next| next.value.contains_key(key))
                    .unwrap_or(false)
            }
        } else if let Some(succ) = desc.successor(byte) {
            if succ.within_range(key) {
                succ.value.contains_key(key)
            } else {
                unsafe { succ.prev.as_ref() }
                    .map(|next| next.value.contains_key(key))
                    .unwrap_or(false)
            }
        } else {
            false
        }
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (byte, desc) = K::lss_longest_descendant_mut(&mut self.lss, key);

        let node_with_successor = if let Some(succ) = desc.successor_mut(byte) {
            if succ.min() <= key || succ.prev.is_null() {
                Some(succ)
            } else if let Some(prev) = unsafe { succ.prev.as_mut() } {
                debug_assert!(prev.value.keys().last().unwrap() < succ.min());
                debug_assert!(prev.key <= key);
                debug_assert!(succ.key > key);
                Some(prev)
            } else {
                unreachable!();
            }
        } else if let Some(pred) = desc.predecessor_mut(byte) {
            if pred.max() >= key || pred.next.is_null() {
                Some(pred)
            } else if let Some(next) = unsafe { pred.next.as_mut() } {
                debug_assert!(next.value.keys().next().unwrap() > pred.max());
                debug_assert!(next.key >= key);
                debug_assert!(pred.key < key);
                Some(next)
            } else {
                unreachable!();
            }
        } else {
            None
        };

        if let Some(node) = node_with_successor {
            let output = node.value.insert(key, value);
            if output.is_none() {
                self.len += 1;
            }
            if node.is_full() {
                let new = node.split();
                self.insert_lss(new);
            }
            return output;
        }

        let mut node = Box::new(LNode::new(key, BTreeRange::new()));
        node.value.insert(key, value);
        self.len += 1;
        self.insert_lss(node);

        None
    }

    fn insert_lss(&mut self, mut node: Box<LinkedBTree<K, V>>) {
        match self.map.entry(node.key) {
            HashEntry::Occupied(mut o) => {
                o.insert(node);
            }
            HashEntry::Vacant(v) => {
                K::lss_insert(&mut self.lss, node.as_mut());
                v.insert(node);
            }
        }
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        let (byte, desc) = K::lss_longest_descendant_mut(&mut self.lss, key);
        let node_with_successor = if let Some(succ) = desc.successor_mut(byte) {
            let min = succ.value.keys().next()?;
            if min <= key || succ.prev.is_null() || succ.key == key {
                Some(succ)
            } else if let Some(prev) = unsafe { succ.prev.as_mut() } {
                debug_assert!(prev.value.keys().last().unwrap() < min);
                debug_assert!(prev.key <= key);
                debug_assert!(succ.key > key);
                Some(prev)
            } else {
                unreachable!();
            }
        } else if let Some(pred) = desc.predecessor_mut(byte) {
            let max = pred.value.keys().last()?;
            if max >= key || pred.next.is_null() || pred.key == key {
                Some(pred)
            } else if let Some(next) = unsafe { pred.next.as_mut() } {
                debug_assert!(next.value.keys().next().unwrap() > max);
                debug_assert!(next.key >= key);
                debug_assert!(pred.key < key);
                Some(next)
            } else {
                unreachable!();
            }
        } else {
            None
        };

        let mut output = None;
        let mut to_remove = None;
        if let Some(node) = node_with_successor {
            output = node.value.remove(key);
            if output.is_some() {
                self.len -= 1;
            }
            if node.is_small() {
                to_remove = Some(node.key);
                let other = node.remove();
                if other.is_full() {
                    let new = other.split();
                    self.insert_lss(new);
                }
            }
        }

        if let Some(key) = to_remove {
            self.remove_lss(key);
        }

        output
    }

    fn remove_lss(&mut self, key: K) {
        if let Some(node) = self.map.remove(&key) {
            K::lss_remove(&mut self.lss, &node);
            unsafe {
                if let Some(prev) = node.prev.as_mut() {
                    prev.next = node.next;
                }
                if let Some(next) = node.next.as_mut() {
                    next.prev = node.prev;
                }
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        let min = K::lss_min(&self.lss);
        Iter {
            btree: min,
            iter: min.map(|m| m.value.iter()),
        }
    }
}

struct Iter<'a, K, V>
where
    K: LevelSearchable<BTreeRange<K, V>>,
{
    btree: Option<&'a LinkedBTree<K, V>>,
    iter: Option<
        std::iter::Zip<
            std::iter::Cloned<std::slice::Iter<'a, K>>,
            std::slice::Iter<'a, V>,
        >,
    >,
}

impl<'a, K, V> Iterator for Iter<'a, K, V>
where
    K: LevelSearchable<BTreeRange<K, V>>,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(iter) = self.iter.as_mut() {
            let output = iter.next();

            if output.is_none() {
                self.btree =
                    self.btree.and_then(|btree| unsafe { btree.next.as_ref() });
                self.iter = self.btree.map(|btree| btree.value.iter());
                return self.next();
            }

            output.map(|(k, v)| (k, v))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_yfast() {
        let items: Vec<u32> = vec![
            2076770906, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15,
            16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 33,
            34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
            51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63,
        ];
        let mut yfast = YFastMap::new();
        let mut expected = std::collections::BTreeSet::new();
        for i in items.iter().cloned() {
            assert_eq!(yfast.insert(i, i), None);
            expected.insert(i);
        }

        let i = 31;
        assert_eq!(yfast.predecessor(32), Some((i, &i)));
    }

    #[test]
    fn test_yfast_get() {
        let mut yfast = YFastMap::new();
        for i in 0..1000u32 {
            assert_eq!(yfast.insert(i, i), None);
        }

        for i in 0..1000u32 {
            assert_eq!(yfast.insert(i, 2 * i), Some(i));
        }

        for i in 0..1000u32 {
            assert_eq!(yfast.get(i), Some(&(2 * i)));
        }

        for i in 1000..2000u32 {
            assert_eq!(yfast.get(i), None);
        }
    }

    #[test]
    fn test_yfast_predecessor_successor() {
        let mut yfast = YFastMap::new();

        for i in 0..1000u32 {
            assert_eq!(yfast.insert(2 * i, i), None);
        }
        yfast.insert(u32::max_value(), 0);

        for i in 0..1000u32 {
            assert_eq!(yfast.predecessor(2 * i), Some((2 * i, &i)));
            assert_eq!(yfast.successor(2 * i), Some((2 * i, &i)));

            assert_eq!(yfast.predecessor(2 * i + 1), Some((2 * i, &i)));

            if i != 999 {
                assert_eq!(
                    yfast.successor(2 * i + 1),
                    Some((2 * i + 2, &(i + 1)))
                );
            }
        }

        // Boundary case
        assert_eq!(yfast.successor(10000), Some((u32::max_value(), &0)));
        assert_eq!(
            yfast.predecessor(u32::max_value()),
            Some((u32::max_value(), &0))
        );
        assert_eq!(
            yfast.successor(u32::max_value()),
            Some((u32::max_value(), &0))
        );
    }

    #[test]
    fn test_yfast_iter() {
        let mut yfast = YFastMap::new();
        let mut expected = Vec::with_capacity(1001);

        for i in 0..1000u32 {
            assert_eq!(yfast.insert(2 * i, i), None);
            expected.push((2 * i, i));
        }
        yfast.insert(u32::max_value(), 0);
        expected.push((u32::max_value(), 0));

        assert_eq!(
            yfast.iter().map(|(k, v)| (k, *v)).collect::<Vec<_>>(),
            expected
        );
    }

    #[test]
    fn test_yfast_remove() {
        let mut yfast = YFastMap::new();
        for i in 0..2000u32 {
            assert_eq!(yfast.insert(i, i), None);
        }
        for i in 0..1000u32 {
            assert_eq!(yfast.remove(2 * i + 1), Some(2 * i + 1));
        }

        assert_eq!(yfast.remove(123213213), None);

        assert_eq!(
            yfast.iter().map(|(k, v)| (k, *v)).collect::<Vec<_>>(),
            (0..1000u32).map(|k| (2 * k, 2 * k)).collect::<Vec<_>>()
        );
    }
}
