use std::collections::btree_map::Entry as BTreeEntry;
use std::collections::BTreeMap;
use std::ops::{Bound, RangeBounds};
use std::ptr;

use fnv::FnvBuildHasher;
use hashbrown::hash_map::Entry as HashEntry;

type HashMap<K, V> = hashbrown::HashMap<K, V, FnvBuildHasher>;

pub struct XFastMap<T> {
    lss: LevelSearch<T>,
    map: HashMap<u32, Box<LNode<T>>>,
}

pub(super) struct Iter<'a, T>(Option<&'a LNode<T>>);
impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (u32, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(node) = self.0 {
            self.0 = unsafe { node.next.as_ref() };
            Some((node.key, &node.value))
        } else {
            None
        }
    }
}

pub(super) struct Range<'a, T, R>
where
    R: RangeBounds<u32>,
{
    range: R,
    node: Option<&'a LNode<T>>,
}
impl<'a, T, R> Iterator for Range<'a, T, R>
where
    R: RangeBounds<u32>,
{
    type Item = (u32, &'a T);

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

impl<T> XFastMap<T> {
    pub fn new() -> XFastMap<T> {
        XFastMap {
            lss: LevelSearch::new(),
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
        self.lss.clear();
        self.map.clear();
    }

    /// Return a reference to the value corresponding to the key
    pub fn get(&self, key: u32) -> Option<&T> {
        self.map.get(&key).map(|node| &node.value)
    }

    pub fn contains_key(&self, key: u32) -> bool {
        self.map.contains_key(&key)
    }

    pub fn insert(&mut self, key: u32, value: T) -> Option<T> {
        match self.map.entry(key) {
            HashEntry::Occupied(mut o) => {
                let node = o.get_mut().as_mut();
                Some(std::mem::replace(&mut node.value, value))
            }
            HashEntry::Vacant(v) => {
                let mut node = Box::new(LNode::new(key, value));
                self.lss.insert(&mut node);
                v.insert(node);
                None
            }
        }
    }

    pub fn remove(&mut self, key: u32) -> Option<T> {
        match self.map.entry(key) {
            HashEntry::Vacant(_) => None,
            HashEntry::Occupied(o) => {
                let node = o.remove();
                self.lss.remove(&node);
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

    pub fn iter(&self) -> impl Iterator<Item = (u32, &T)> {
        Iter(self.lss.min())
    }

    pub fn range(
        &self,
        range: impl RangeBounds<u32>,
    ) -> impl Iterator<Item = (u32, &T)> {
        if self.is_empty() {
            return Range { range, node: None };
        }

        let node = match range.start_bound() {
            Bound::Unbounded => self.lss.min(),
            Bound::Included(&key) => self.lss.successor(key),
            Bound::Excluded(&key) => {
                if key == u32::max_value() {
                    None
                } else {
                    self.lss.successor(key + 1)
                }
            }
        };
        Range { range, node }
    }

    pub fn predecessor(&self, key: u32) -> Option<(u32, &T)> {
        self.lss
            .predecessor(key)
            .map(|node| (node.key, &node.value))
    }

    pub fn successor(&self, key: u32) -> Option<(u32, &T)> {
        self.lss.successor(key).map(|node| (node.key, &node.value))
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LevelSearch<T> {
    l0: Descendant<T>,
    l1: HashMap<[u8; 1], Descendant<T>>,
    l2: HashMap<[u8; 2], Descendant<T>>,
    l3: HashMap<[u8; 3], Descendant<T>>,
}

impl<T> LevelSearch<T> {
    fn new() -> LevelSearch<T> {
        LevelSearch {
            l0: Descendant::new(),
            l1: HashMap::default(),
            l2: HashMap::default(),
            l3: HashMap::default(),
        }
    }

    fn clear(&mut self) {
        self.l0 = Descendant::new();
        self.l1.clear();
        self.l2.clear();
        self.l3.clear();
    }

    fn insert(&mut self, node: &mut LNode<T>) {
        let bytes = node.key.to_be_bytes();
        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];

        // Do not use longest_descendant so we can re-use the hashes and
        // entries
        let mut v1 = self.l1.entry(b1);
        let mut v2 = self.l2.entry(b2);
        let mut v3 = self.l3.entry(b3);

        if let HashEntry::Occupied(ref mut o) = v2 {
            if let HashEntry::Occupied(ref mut o) = v3 {
                o.get_mut().set_links(bytes[3], node);
            } else {
                o.get_mut().set_links(bytes[2], node);
            }
        } else {
            if let HashEntry::Occupied(ref mut o) = v1 {
                o.get_mut().set_links(bytes[1], node);
            } else {
                self.l0.set_links(bytes[0], node);
            }
        }

        self.l0.merge(bytes[0], node);

        fn insert_into_entry<T, K: std::hash::Hash>(
            byte: u8,
            node: &mut LNode<T>,
            entry: HashEntry<K, Descendant<T>, FnvBuildHasher>,
        ) {
            match entry {
                HashEntry::Vacant(v) => {
                    let mut desc = Descendant::new();
                    desc.bounds.insert(byte, unsafe {
                        (
                            ptr::NonNull::new_unchecked(node),
                            ptr::NonNull::new_unchecked(node),
                        )
                    });
                    v.insert(desc);
                }
                HashEntry::Occupied(mut o) => {
                    o.get_mut().merge(byte, node);
                }
            }
        }
        insert_into_entry(bytes[1], node, v1);
        insert_into_entry(bytes[2], node, v2);
        insert_into_entry(bytes[3], node, v3);
    }

    fn remove(&mut self, node: &LNode<T>) {
        let bytes = node.key.to_be_bytes();
        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];

        self.l0.remove(bytes[0], node);
        if let HashEntry::Occupied(mut o) = self.l1.entry(b1) {
            o.get_mut().remove(bytes[1], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let HashEntry::Occupied(mut o) = self.l2.entry(b2) {
            o.get_mut().remove(bytes[2], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let HashEntry::Occupied(mut o) = self.l3.entry(b3) {
            o.get_mut().remove(bytes[3], node);
        }
    }

    fn min(&self) -> Option<&LNode<T>> {
        let (_, desc) = self.longest_descendant(0);
        desc.successor(0)
    }

    fn predecessor(&self, key: u32) -> Option<&LNode<T>> {
        let (byte, desc) = self.longest_descendant(key);
        desc.predecessor(byte).or_else(|| {
            desc.successor(byte)
                .and_then(|node| unsafe { node.prev.as_ref() })
        })
    }

    fn successor(&self, key: u32) -> Option<&LNode<T>> {
        let (byte, desc) = self.longest_descendant(key);
        desc.successor(byte).or_else(|| {
            desc.predecessor(byte)
                .and_then(|node| unsafe { node.next.as_ref() })
        })
    }

    fn longest_descendant(&self, key: u32) -> (u8, &Descendant<T>) {
        let bytes = key.to_be_bytes();
        if let Some(desc) = self.l2.get(&[bytes[0], bytes[1]]) {
            if let Some(desc) = self.l3.get(&[bytes[0], bytes[1], bytes[2]]) {
                (bytes[3], &desc)
            } else {
                (bytes[2], &desc)
            }
        } else {
            if let Some(desc) = self.l1.get(&[bytes[0]]) {
                (bytes[1], &desc)
            } else {
                (bytes[0], &self.l0)
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Descendant<T> {
    bounds: BTreeMap<u8, (ptr::NonNull<LNode<T>>, ptr::NonNull<LNode<T>>)>,
}

impl<T> Descendant<T> {
    fn new() -> Descendant<T> {
        Descendant {
            bounds: BTreeMap::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    /// Find the predecessor of byte, assuming byte has at most 1 child
    fn predecessor(&self, byte: u8) -> Option<&LNode<T>> {
        self.bounds
            .range(0..=byte)
            .rev()
            .next()
            .map(|(&b, (min, max))| {
                if b == byte {
                    debug_assert_eq!(min, max);
                }
                unsafe { max.as_ref() }
            })
    }

    /// Find the predecessor of byte, assuming byte has at most 1 child
    fn predecessor_mut(&mut self, byte: u8) -> Option<&mut LNode<T>> {
        self.bounds
            .range_mut(0..=byte)
            .rev()
            .next()
            .map(|(&b, (min, max))| {
                if b == byte {
                    debug_assert_eq!(min, max);
                }
                unsafe { max.as_mut() }
            })
    }

    /// Find the successor of byte, assuming byte has at most 1 child
    fn successor_mut(&mut self, byte: u8) -> Option<&mut LNode<T>> {
        self.bounds
            .range_mut(byte..)
            .next()
            .map(|(&b, (min, max))| {
                if b == byte {
                    debug_assert_eq!(min, max);
                }
                unsafe { min.as_mut() }
            })
    }

    /// Find the successor of byte, assuming byte has at most 1 child
    fn successor(&self, byte: u8) -> Option<&LNode<T>> {
        self.bounds.range(byte..).next().map(|(&b, (min, max))| {
            if b == byte {
                debug_assert_eq!(min, max);
            }
            unsafe { min.as_ref() }
        })
    }

    /// If this is the "lowest" Descendant matching the prefix, insert
    /// node into the linked list.
    fn set_links(&mut self, byte: u8, node: &mut LNode<T>) {
        if let Some(next) = self.successor_mut(byte) {
            debug_assert!(next.key > node.key);
            node.set_next(next);
        } else if let Some(prev) = self.predecessor_mut(byte) {
            debug_assert!(prev.key < node.key);
            node.set_prev(prev);
        }
    }

    fn merge(&mut self, byte: u8, node: &mut LNode<T>) {
        match self.bounds.entry(byte) {
            BTreeEntry::Vacant(v) => {
                v.insert(unsafe {
                    (
                        ptr::NonNull::new_unchecked(node),
                        ptr::NonNull::new_unchecked(node),
                    )
                });
            }
            BTreeEntry::Occupied(mut o) => {
                let (min, max) = o.get_mut();
                if node.key < unsafe { min.as_ref() }.key {
                    *min = ptr::NonNull::from(node);
                } else if node.key > unsafe { max.as_ref() }.key {
                    *max = ptr::NonNull::from(node);
                }
            }
        }
    }

    /// Remove the byte/node pair from the descendant pointers
    fn remove(&mut self, byte: u8, node: &LNode<T>) {
        match self.bounds.entry(byte) {
            BTreeEntry::Occupied(mut o) => {
                let (min, max) = o.get_mut();

                if ptr::eq(min.as_ptr(), node) {
                    if ptr::eq(max.as_ptr(), node) {
                        // (min == max == node) => node is only entry
                        o.remove();
                    } else {
                        *min =
                            unsafe { ptr::NonNull::new_unchecked(node.next) };
                    }
                } else if ptr::eq(max.as_ptr(), node) {
                    *max = unsafe { ptr::NonNull::new_unchecked(node.prev) };
                }
            }
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct LNode<T> {
    key: u32,
    value: T,

    prev: *mut LNode<T>,
    next: *mut LNode<T>,
}

impl<T> LNode<T> {
    fn new(key: u32, value: T) -> LNode<T> {
        LNode {
            key,
            value,
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }

    fn set_prev(&mut self, other: &mut LNode<T>) {
        self.prev = other;
        self.next = other.next;

        other.next = self;
        if let Some(next) = unsafe { self.next.as_mut() } {
            next.prev = self;
        }
    }

    fn set_next(&mut self, other: &mut LNode<T>) {
        self.next = other;
        self.prev = other.prev;

        other.prev = self;
        if let Some(prev) = unsafe { self.prev.as_mut() } {
            prev.next = self;
        }
    }
}

#[cfg(test)]
mod test {
    use std::iter::FromIterator;

    use super::*;

    #[test]
    fn test_levelsearch_insert_1() {
        let mut lss = LevelSearch::new();
        let mut node = LNode::new(0xdeadbeef, ());

        lss.insert(&mut node);

        let ptr = &mut node as *mut _;
        let nonnull = ptr::NonNull::new(ptr).unwrap();

        assert_eq!(
            lss.l0.bounds,
            BTreeMap::from_iter(vec![(0xde, (nonnull, nonnull)),])
        );

        assert_eq!(
            lss.l1,
            HashMap::from_iter(vec![(
                [0xde],
                Descendant {
                    bounds: BTreeMap::from_iter(vec![(
                        0xad,
                        (nonnull, nonnull)
                    ),])
                }
            )])
        );

        assert_eq!(
            lss.l2,
            HashMap::from_iter(vec![(
                [0xde, 0xad],
                Descendant {
                    bounds: BTreeMap::from_iter(vec![(
                        0xbe,
                        (nonnull, nonnull)
                    ),])
                }
            )])
        );

        assert_eq!(
            lss.l3,
            HashMap::from_iter(vec![(
                [0xde, 0xad, 0xbe],
                Descendant {
                    bounds: BTreeMap::from_iter(vec![(
                        0xef,
                        (nonnull, nonnull)
                    ),])
                }
            )])
        );
    }

    #[test]
    fn test_levelsearch_insert_4() {
        let mut lss = LevelSearch::new();
        let mut n1 = LNode::new(0xbaadf00d, ());
        let mut n2 = LNode::new(0xdeadbeef, ());
        let mut n3 = LNode::new(0xdeadc0de, ());
        let mut n4 = LNode::new(0xdeadc0fe, ());

        lss.insert(&mut n3);
        lss.insert(&mut n4);
        lss.insert(&mut n2);
        lss.insert(&mut n1);

        let p1 = &mut n1 as *mut _;
        let p2 = &mut n2 as *mut _;
        let p3 = &mut n3 as *mut _;
        let p4 = &mut n4 as *mut _;

        assert!(n1.prev.is_null());
        assert_eq!(n1.next, p2);
        assert_eq!(n2.prev, p1);
        assert_eq!(n2.next, p3);
        assert_eq!(n3.prev, p2);
        assert_eq!(n3.next, p4);
        assert_eq!(n4.prev, p3);
        assert!(n4.next.is_null());

        let nn1 = ptr::NonNull::new(p1).unwrap();
        let nn2 = ptr::NonNull::new(p2).unwrap();
        let nn3 = ptr::NonNull::new(p3).unwrap();
        let nn4 = ptr::NonNull::new(p4).unwrap();

        assert_eq!(
            lss.l0.bounds,
            BTreeMap::from_iter(vec![(0xba, (nn1, nn1)), (0xde, (nn2, nn4))])
        );
        assert_eq!(
            lss.l1,
            HashMap::from_iter(vec![
                (
                    [0xba],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xad, (nn1, nn1))])
                    }
                ),
                (
                    [0xde],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xad, (nn2, nn4))])
                    }
                )
            ])
        );

        assert_eq!(
            lss.l2,
            HashMap::from_iter(vec![
                (
                    [0xba, 0xad],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xf0, (nn1, nn1))])
                    }
                ),
                (
                    [0xde, 0xad],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![
                            (0xbe, (nn2, nn2)),
                            (0xc0, (nn3, nn4)),
                        ])
                    }
                )
            ])
        );
        assert_eq!(
            lss.l3,
            HashMap::from_iter(vec![
                (
                    [0xba, 0xad, 0xf0],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0x0d, (nn1, nn1))])
                    }
                ),
                (
                    [0xde, 0xad, 0xbe],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xef, (nn2, nn2))])
                    }
                ),
                (
                    [0xde, 0xad, 0xc0],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![
                            (0xde, (nn3, nn3)),
                            (0xfe, (nn4, nn4)),
                        ])
                    }
                ),
            ])
        );
    }

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

            let mut n = xfast.lss.min().unwrap();

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
