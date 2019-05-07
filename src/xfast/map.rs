use std::collections::hash_map::Entry as HashEntry;
use std::ops::{Bound, RangeBounds};

use fnv::FnvHashMap as HashMap;

use super::{traits::LNode, LevelSearchable};

#[derive(Default)]
pub struct XFastMap<K: LevelSearchable<V>, V> {
    lss: K::LSS,
    map: HashMap<K, Box<LNode<K, V>>>,
}

pub(super) struct Iter<'a, K: LevelSearchable<V>, V>(Option<&'a LNode<K, V>>);
impl<'a, K: LevelSearchable<V>, V> Iterator for Iter<'a, K, V> {
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.0 {
            self.0 = unsafe { node.next.as_ref() };
            Some((node.key, &node.value))
        } else {
            None
        }
    }
}

pub(super) struct Range<'a, K: LevelSearchable<V>, V, R>
where
    R: RangeBounds<K>,
{
    range: R,
    node: Option<&'a LNode<K, V>>,
}
impl<'a, K: LevelSearchable<V>, V, R> Iterator for Range<'a, K, V, R>
where
    R: RangeBounds<K>,
{
    type Item = (K, &'a V);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.node {
            let contains = match self.range.end_bound() {
                Bound::Unbounded => true,
                Bound::Excluded(upper) => node.key < *upper,
                Bound::Included(upper) => node.key <= *upper,
            };

            if contains {
                self.node = unsafe { node.next.as_ref() };
                Some((node.key, &node.value))
            } else {
                self.node = None;
                None
            }
        } else {
            None
        }
    }
}

impl<K: LevelSearchable<V>, V> XFastMap<K, V> {
    pub fn new() -> XFastMap<K, V> {
        XFastMap {
            lss: K::lss_new(),
            map: HashMap::default(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Clear the map, removing all keys and values
    pub fn clear(&mut self) {
        K::lss_clear(&mut self.lss);
        self.map.clear();
    }

    /// Return a reference to the value corresponding to the key
    pub fn get(&self, key: K) -> Option<&V> {
        self.map.get(&key).map(|node| &node.value)
    }

    pub fn contains_key(&self, key: K) -> bool {
        self.map.contains_key(&key)
    }

    pub fn insert(&mut self, key: K, value: V) -> Option<V> {
        match self.map.entry(key) {
            HashEntry::Occupied(mut o) => {
                let node = o.get_mut().as_mut();
                Some(std::mem::replace(&mut node.value, value))
            }
            HashEntry::Vacant(v) => {
                let mut node = Box::new(LNode::new(key, value));
                K::lss_insert(&mut self.lss, &mut node);
                v.insert(node);
                None
            }
        }
    }

    pub fn remove(&mut self, key: K) -> Option<V> {
        match self.map.entry(key) {
            HashEntry::Vacant(_) => None,
            HashEntry::Occupied(o) => {
                let node = o.remove();
                K::lss_remove(&mut self.lss, &node);
                unsafe {
                    if let Some(prev) = node.prev.as_mut() {
                        prev.next = node.next;
                    }
                    if let Some(next) = node.next.as_mut() {
                        next.prev = node.prev;
                    }
                }
                Some(node.value)
            }
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (K, &V)> {
        Iter(K::lss_min(&self.lss))
    }

    pub fn range(
        &self,
        range: impl RangeBounds<K>,
    ) -> impl Iterator<Item = (K, &V)> {
        if self.is_empty() {
            return Range { range, node: None };
        }

        let node = match range.start_bound() {
            Bound::Unbounded => K::lss_min(&self.lss),
            Bound::Included(&key) => K::lss_successor(&self.lss, key),
            Bound::Excluded(&key) => {
                if key == K::MAX {
                    None
                } else {
                    unimplemented!();
                    // self.lss.successor(key + 1)
                }
            }
        };
        Range { range, node }
    }

    pub fn predecessor(&self, key: K) -> Option<(K, &V)> {
        K::lss_predecessor(&self.lss, key).map(|node| (node.key, &node.value))
    }

    pub fn successor(&self, key: K) -> Option<(K, &V)> {
        K::lss_successor(&self.lss, key).map(|node| (node.key, &node.value))
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use super::super::traits::LevelSearchable;

    #[test]
    fn test_xfast_iter() {
        let keys: [u32; 34] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7, 0, 0xFFFFFFFF,
        ];

        let mut xfast = XFastMap::new();
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(xfast.insert(*key, ()), None);
            let mut sorted =
                keys[..=i].iter().map(|k| (*k, &())).collect::<Vec<_>>();
            sorted.sort();
            assert_eq!(xfast.iter().collect::<Vec<_>>(), sorted,);
        }
    }

    #[test]
    fn test_xfast_range() {
        let mut keys: [u32; 34] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7, 0, 0xFFFFFFFF,
        ];

        let mut xfast = XFastMap::new();
        for key in keys.iter().cloned() {
            assert_eq!(xfast.insert(key, ()), None);
        }

        keys.sort();
        for i in 0..keys.len() {
            // (Unbounded, Exclusive)
            let range = xfast.range(..keys[i]);
            assert_eq!(
                &range.map(|k| k.0).collect::<Vec<_>>() as &[u32],
                &keys[..i]
            );

            // (Unbounded, Inclusive)
            let range = xfast.range(..=keys[i]);
            assert_eq!(
                &range.map(|k| k.0).collect::<Vec<_>>() as &[u32],
                &keys[..=i]
            );

            // (Inclusive, Bounded)
            let range = xfast.range(keys[i]..);
            assert_eq!(
                &range.map(|k| k.0).collect::<Vec<_>>() as &[u32],
                &keys[i..]
            );

            for j in i..keys.len() {
                // (Inclusive, Exclusive)
                let range = xfast.range(keys[i]..keys[j]);
                assert_eq!(
                    &range.map(|k| k.0).collect::<Vec<_>>() as &[u32],
                    &keys[i..j]
                );

                // (Inclusive, Inclusive)
                let range = xfast.range(keys[i]..=keys[j]);
                assert_eq!(
                    &range.map(|k| k.0).collect::<Vec<_>>() as &[u32],
                    &keys[i..=j]
                );
            }
        }
    }

    #[test]
    fn test_xfast_insert_preserves_linked_list() {
        let keys: [u32; 34] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7, 0, 0xFFFFFFFF,
        ];

        let mut xfast = XFastMap::new();
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(xfast.insert(*key, ()), None);

            let mut sorted = keys[..=i].iter().cloned().collect::<Vec<_>>();
            sorted.sort();

            let mut n = u32::lss_min(&xfast.lss).unwrap();

            for j in 0..i {
                assert_eq!(n.key, sorted[j]);
                n = unsafe { n.next.as_ref().unwrap() }
            }
            assert_eq!(n.key, sorted[i]);
            assert!(n.next.is_null());
        }
    }

    #[test]
    fn test_xfast_predecessor_successor() {
        let keys: [u32; 34] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7, 0, 0xFFFFFFFF,
        ];

        let mut xfast = XFastMap::new();
        for (i, key) in keys.iter().enumerate() {
            assert_eq!(xfast.insert(*key, ()), None);

            let mut sorted = keys[..=i].iter().cloned().collect::<Vec<_>>();
            sorted.sort();
            for (j, ki) in sorted.iter().cloned().enumerate() {
                if j > 0 {
                    assert_eq!(
                        xfast.predecessor(ki - 1),
                        Some((sorted[j - 1], &()))
                    );
                }
                assert_eq!(xfast.predecessor(ki), Some((sorted[j], &())));
                assert_eq!(xfast.successor(ki), Some((sorted[j], &())));

                if j + 1 < sorted.len() {
                    assert_eq!(
                        xfast.successor(ki + 1),
                        Some((sorted[j + 1], &()))
                    );
                }
            }
        }
    }

    #[test]
    fn test_xfast_integration_remove() {
        let keys: [u32; 34] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7, 0, 0xFFFFFFFF,
        ];
        let mut xfast = XFastMap::new();
        for key in keys.iter().cloned() {
            assert_eq!(xfast.insert(key, ()), None);
        }

        for (i, key) in keys.iter().cloned().enumerate() {
            assert_eq!(xfast.remove(key), Some(()));
            assert_eq!(xfast.remove(key), None);

            // Check that predecessor works for each sorted element
            let mut sorted = keys[1 + i..].iter().cloned().collect::<Vec<_>>();
            sorted.sort();
            for (j, ki) in sorted.iter().cloned().enumerate() {
                assert_eq!(xfast.predecessor(ki), Some((ki, &())));
                assert_eq!(xfast.successor(ki), Some((ki, &())));

                if ki < u32::max_value() {
                    assert_eq!(xfast.predecessor(ki + 1), Some((ki, &())));
                    if j + 1 < sorted.len() {
                        assert_eq!(
                            xfast.successor(ki + 1),
                            Some((sorted[j + 1], &()))
                        );
                    }
                }
                if ki > 0 {
                    assert_eq!(xfast.successor(ki - 1), Some((ki, &())));

                    if j > 0 {
                        assert_eq!(
                            xfast.predecessor(ki - 1),
                            Some((sorted[j - 1], &()))
                        );
                    }
                }
            }
        }
    }
}
