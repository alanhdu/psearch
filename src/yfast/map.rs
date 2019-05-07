use std::collections::{
    btree_map::Iter as BTreeIter, hash_map::Entry as HashEntry, BTreeMap,
};

use fnv::FnvHashMap as HashMap;

use super::{LevelSearchable, LinkedBTree};
use crate::level_search::LNode;

#[derive(Default, Debug)]
pub struct YFastMap<K: LevelSearchable<BTreeMap<K, V>>, V> {
    lss: K::LSS,
    map: HashMap<K, Box<LinkedBTree<K, V>>>,
    len: usize,
}

impl<K: LevelSearchable<BTreeMap<K, V>>, V> YFastMap<K, V> {
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
            pred.value.get(&key).or_else(|| {
                unsafe { pred.next.as_ref() }
                    .and_then(|next| next.value.get(&key))
            })
        } else if let Some(succ) = desc.successor(byte) {
            succ.value.get(&key).or_else(|| {
                unsafe { succ.prev.as_ref() }
                    .and_then(|prev| prev.value.get(&key))
            })
        } else {
            None
        }
    }

    pub fn predecessor(&self, key: K) -> Option<(K, &V)> {
        let (byte, desc) = K::lss_longest_descendant(&self.lss, key);
        if let Some(pred) = desc.predecessor(byte) {
            unsafe { pred.next.as_ref() }
                .and_then(|next| next.value.range(..=key).rev().next())
                .or_else(|| pred.value.range(..=key).rev().next())
                .map(|(k, v)| (*k, v))
        } else if let Some(succ) = desc.successor(byte) {
            succ.value
                .range(..=key)
                .next_back()
                .or_else(|| {
                    unsafe { succ.prev.as_ref() }
                        .and_then(|prev| prev.value.range(..=key).rev().next())
                })
                .map(|(k, v)| (*k, v))
        } else {
            None
        }
    }

    pub fn successor(&self, key: K) -> Option<(K, &V)> {
        let (byte, desc) = K::lss_longest_descendant(&self.lss, key);
        if let Some(pred) = desc.predecessor(byte) {
            pred.value
                .range(key..)
                .next()
                .or_else(|| {
                    unsafe { pred.next.as_ref() }
                        .and_then(|next| next.value.range(key..).next())
                })
                .map(|(k, v)| (*k, v))
        } else if let Some(succ) = desc.successor(byte) {
            unsafe { succ.prev.as_ref() }
                .and_then(|prev| prev.value.range(key..).next())
                .or_else(|| succ.value.range(key..).next())
                .map(|(k, v)| (*k, v))
        } else {
            None
        }
    }

    pub fn contains_key(&self, key: K) -> bool {
        if let Some(pred) = K::lss_predecessor(&self.lss, key) {
            pred.value.contains_key(&key)
                || unsafe { pred.next.as_ref() }
                    .map(|next| next.value.contains_key(&key))
                    .unwrap_or(false)
        } else if let Some(succ) = K::lss_successor(&self.lss, key) {
            succ.value.contains_key(&key)
                || unsafe { succ.prev.as_ref() }
                    .map(|next| next.value.contains_key(&key))
                    .unwrap_or(false)
        } else {
            false
        }
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

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        let (byte, desc) = K::lss_longest_descendant_mut(&mut self.lss, key);

        if let Some(succ) = desc.successor_mut(byte) {
            let min = succ.value.keys().next().unwrap();
            if *min <= key || succ.prev.is_null() || succ.key == key {
                let output = succ.value.insert(key, value);
                if succ.is_full() {
                    let new = succ.split();
                    self.insert_lss(new);
                }
                return output;
            } else if let Some(prev) = unsafe { succ.prev.as_mut() } {
                debug_assert!(prev.value.keys().rev().next().unwrap() < min);
                debug_assert!(prev.key <= key);
                debug_assert!(succ.key > key);
                let output = prev.value.insert(key, value);
                if prev.is_full() {
                    let new = prev.split();
                    self.insert_lss(new);
                }
                return output;
            } else {
                unreachable!();
            }
        } else if let Some(pred) = desc.predecessor_mut(byte) {
            let max = pred.value.keys().rev().next().unwrap();
            if *max >= key || pred.next.is_null() || pred.key == key {
                let output = pred.value.insert(key, value);
                if pred.is_full() {
                    let new = pred.split();
                    self.insert_lss(new);
                }
                return output;
            } else if let Some(next) = unsafe { pred.next.as_mut() } {
                debug_assert!(next.value.keys().next().unwrap() > max);
                debug_assert!(next.key >= key);
                debug_assert!(pred.key < key);
                let output = next.value.insert(key, value);
                if next.is_full() {
                    let new = next.split();
                    self.insert_lss(new);
                }
                return output;
            } else {
                unreachable!();
            }
        }

        let mut node = Box::new(LNode::new(key, BTreeMap::new()));
        node.value.insert(key, value);
        self.insert_lss(node);

        None
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        let min = K::lss_min(&self.lss);
        Iter {
            btree: min,
            iter: min.map(|m| m.value.iter()),
        }
    }
}

struct Iter<'a, K: LevelSearchable<BTreeMap<K, V>>, V> {
    btree: Option<&'a LinkedBTree<K, V>>,
    iter: Option<BTreeIter<'a, K, V>>,
}

impl<'a, K: LevelSearchable<BTreeMap<K, V>>, V> Iterator for Iter<'a, K, V> {
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

            output.map(|(k, v)| (*k, v))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

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

}
