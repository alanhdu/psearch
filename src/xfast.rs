use std::collections::{hash_map::Entry, HashMap};
use std::ptr;

#[derive(Debug, Eq, PartialEq)]
pub struct XFastMap<T> {
    lss: LevelSearch<T>,
    map: HashMap<u32, ptr::NonNull<LNode<u32, T>>>,
}

fn upper_bits(key: u32, n: usize) -> u32 {
    // Extract upper bits from key
    key & (0xFFFF_FFFF << (31 - n))
}

impl<T: std::fmt::Debug> XFastMap<T> {
    pub fn new() -> XFastMap<T> {
        XFastMap {
            map: HashMap::new(),
            lss: LevelSearch::new(),
        }
    }

    /// Clear the map, removing all keys and values
    pub fn clear(&mut self) {
        self.lss = LevelSearch::new();
        for (_key, value) in self.map.drain() {
            unsafe {
                drop(Box::from_raw(value.as_ptr()));
            }
        }
    }

    /// Return a reference to the value corresponding to the key
    pub fn get(&self, key: u32) -> Option<&T> {
        self.map
            .get(&key)
            .map(|node| &unsafe { node.as_ref() }.value)
    }

    /// Return a reference to the value corresponding to the key
    pub fn get_mut(&mut self, key: u32) -> Option<&mut T> {
        self.map
            .get_mut(&key)
            .map(|node| &mut unsafe { node.as_mut() }.value)
    }

    /// Return a reference to the value corresponding to the key
    pub fn contains_key(&self, key: u32) -> bool {
        self.map.contains_key(&key)
    }

    pub fn insert(&mut self, key: u32, value: T) -> Option<T> {
        match self.map.entry(key) {
            Entry::Occupied(mut o) => {
                let node = unsafe { o.get_mut().as_mut() };
                Some(std::mem::replace(&mut node.value, value))
            }
            Entry::Vacant(v) => {
                let node = self.lss.insert(key, value);
                v.insert(node);
                None
            }
        }
    }

    pub fn remove(&mut self, key: u32) -> Option<T> {
        match self.map.entry(key) {
            Entry::Vacant(_) => None,
            Entry::Occupied(o) => {
                let mut ptr = o.remove();
                let node = unsafe { Box::from_raw(ptr.as_mut()) };
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

    fn predecessor(&self, key: u32) -> Option<&T> {
        // If we are empty, short-circuit evaluation
        if self.map.is_empty() {
            return None;
        }
        // Special-case: if key is in the map
        if let Some(node) = self.map.get(&key) {
            let node = unsafe { node.as_ref() };
            debug_assert!(node.key == key);
            return Some(&node.value);
        }

        match self.lss.longest_descendant(key) {
            Descendant::Both => unreachable!(),
            Descendant::One { min } => {
                let prev = unsafe { min.as_ref().prev.as_ref() }?;
                Some(&prev.value)
            }
            Descendant::Zero { max } => {
                let max = unsafe { max.as_ref() };
                Some(&max.value)
            }
        }
    }
}

impl<T> Drop for XFastMap<T> {
    fn drop(&mut self) {
        for (_key, value) in self.map.drain() {
            unsafe {
                drop(Box::from_raw(value.as_ptr()));
            }
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LevelSearch<T> {
    maps: [HashMap<u32, Descendant<T>>; 31],
    root: Option<Descendant<T>>,
}

impl<T: std::fmt::Debug> LevelSearch<T> {
    fn new() -> LevelSearch<T> {
        LevelSearch {
            maps: arr_macro::arr![HashMap::new(); 31],
            root: None,
        }
    }

    /// Insert into the LevelSearch struct. Return pointer to the LNode
    fn insert(&mut self, key: u32, value: T) -> ptr::NonNull<LNode<u32, T>> {
        let node = unsafe {
            ptr::NonNull::new_unchecked(Box::into_raw(Box::new(LNode {
                key,
                value,
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            })))
        };

        for (i, map) in self.maps.iter_mut().enumerate().rev() {
            let next_bit = key & (1 << (31 - i - 1));
            let child = if next_bit == 0 {
                Descendant::Zero { max: node }
            } else {
                Descendant::One { min: node }
            };

            match map.entry(upper_bits(key, i)) {
                Entry::Vacant(v) => {
                    v.insert(child);
                }
                Entry::Occupied(mut o) => {
                    o.get_mut().merge(child, node);
                }
            }
        }

        let high_bit = key & (1 << 31);
        let child = if high_bit == 0 {
            Descendant::Zero { max: node }
        } else {
            Descendant::One { min: node }
        };

        if let Some(ref mut root) = self.root {
            root.merge(child, node);
        } else {
            self.root = Some(child);
        }
        node
    }

    /// Remove an LNode from the LSS
    ///
    /// Note that this assumes that `node ` is actually *in* the LSS and
    /// will panic otherwise
    fn remove(&mut self, node: &LNode<u32, T>) {
        fn common_prefix_len(a: u32, b: u32) -> u32 {
            (a ^ b).leading_zeros()
        }
        let max_prefix_len = unsafe {
            match (node.next.as_ref(), node.prev.as_ref()) {
                (None, None) => {
                    self.root = None;
                    self.maps.iter_mut().for_each(HashMap::clear);
                    return;
                }
                (Some(next), None) => common_prefix_len(next.key, node.key),
                (None, Some(prev)) => common_prefix_len(prev.key, node.key),
                (Some(next), Some(prev)) => std::cmp::max(
                    common_prefix_len(prev.key, node.key),
                    common_prefix_len(next.key, node.key),
                ),
            }
        } as usize;

        let desc = if max_prefix_len == 0 {
            self.root.as_mut().unwrap()
        } else {
            self.maps[max_prefix_len - 1]
                .get_mut(&upper_bits(node.key, max_prefix_len - 1))
                .unwrap()
        };
        match desc {
            Descendant::Both => unsafe {
                let high_bit = node.key & (1 << (31 - max_prefix_len));
                *desc = if high_bit == 0 {
                    debug_assert!(!node.next.is_null());
                    Descendant::One {
                        min: ptr::NonNull::new_unchecked(node.next),
                    }
                } else {
                    debug_assert!(!node.prev.is_null());
                    Descendant::Zero {
                        max: ptr::NonNull::new_unchecked(node.prev),
                    }
                }
            },
            _ => {
                debug_assert_eq!(max_prefix_len, 0);
                self.root = None;
            }
        }

        if let Some(ref mut desc) = self.root {
            unsafe { desc.remove(node) };
        }
        for i in 0..max_prefix_len {
            match self.maps[i].get_mut(&upper_bits(node.key, i)) {
                Some(desc) => unsafe { desc.remove(node) },
                None => unreachable!(),
            }
        }

        for i in max_prefix_len..self.maps.len() {
            self.maps[i].remove(&upper_bits(node.key, i));
        }
    }

    fn longest_descendant(&self, key: u32) -> &Descendant<T> {
        let mut bit = key & (0b1 << 31);
        let mut level = match self.maps[0].get(&bit) {
            None => {
                bit ^= 0b1 << 31;
                self.maps[0].get(&bit).unwrap()
            }
            Some(p) => p,
        };

        let mut left = 1;
        let mut right = 32 - 2;
        while left <= right {
            let mid = (left + right) / 2;
            match self.maps[mid].get(&upper_bits(key, mid)) {
                None => right = mid - 1,
                Some(m) => {
                    level = m;
                    left = mid + 1;
                }
            }
        }

        if let Descendant::Both = level {
            level = self.root.as_ref().unwrap();
        }
        level
    }

    fn entry(&mut self, key: u32, level: usize) -> Entry<u32, Descendant<T>> {
        debug_assert!(level < 32);
        self.maps[level].entry(upper_bits(key, level))
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Descendant<T> {
    Both,
    Zero { max: ptr::NonNull<LNode<u32, T>> },
    One { min: ptr::NonNull<LNode<u32, T>> },
}

impl<T> Descendant<T> {
    unsafe fn remove(&mut self, node: &LNode<u32, T>) {
        match self {
            Descendant::One { ref mut min } => {
                if ptr::eq(min.as_ref(), node) {
                    debug_assert!(!node.next.is_null());
                    *min = ptr::NonNull::new_unchecked(node.next);
                }
            }
            Descendant::Zero { ref mut max } => {
                if ptr::eq(max.as_ref(), node) {
                    debug_assert!(!node.prev.is_null());
                    *max = ptr::NonNull::new_unchecked(node.prev);
                }
            }
            _ => {}
        }
    }

    fn merge(
        &mut self,
        other: Descendant<T>,
        mut node: ptr::NonNull<LNode<u32, T>>,
    ) {
        let old = std::mem::replace(self, Descendant::Both);
        let replacement = match (old, other) {
            (Descendant::Both, _) => Descendant::Both,
            (_, Descendant::Both) => Descendant::Both,
            (Descendant::Zero { max: mut m }, Descendant::One { .. }) => {
                unsafe {
                    node.as_mut().prev = m.as_ptr();
                    node.as_mut().next = m.as_mut().next;

                    m.as_mut().next = node.as_ptr();
                    if let Some(next) = node.as_mut().next.as_mut() {
                        next.prev = node.as_ptr();
                    }
                }
                Descendant::Both
            }
            (Descendant::One { min: mut m }, Descendant::Zero { .. }) => {
                unsafe {
                    node.as_mut().next = m.as_ptr();
                    node.as_mut().prev = m.as_mut().prev;
                    m.as_mut().prev = node.as_ptr();

                    if let Some(prev) = node.as_mut().prev.as_mut() {
                        prev.next = node.as_ptr();
                    }
                }
                Descendant::Both
            }
            (Descendant::Zero { max: m1 }, Descendant::Zero { max: m2 }) => {
                if unsafe { m2.as_ref().key > m1.as_ref().key } {
                    Descendant::Zero { max: m2 }
                } else {
                    Descendant::Zero { max: m1 }
                }
            }
            (Descendant::One { min: m1 }, Descendant::One { min: m2 }) => {
                if unsafe { m2.as_ref().key < m1.as_ref().key } {
                    Descendant::One { min: m2 }
                } else {
                    Descendant::One { min: m1 }
                }
            }
        };
        std::mem::replace(self, replacement);
    }
}

#[derive(Debug, Eq, PartialEq)]
struct LNode<K: Eq + PartialEq, V> {
    key: K,
    value: V,

    prev: *mut LNode<K, V>,
    next: *mut LNode<K, V>,
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_upper_bits() {
        let key = 0b10000110000100111001001010010001;
        for i in 0..31 {
            let upper = upper_bits(key, i);
            assert_eq!(upper & key, upper);
        }
        assert_eq!(upper_bits(key, 0), 0b1 << 31);
        assert_eq!(upper_bits(key, 1), 0b10 << 30);
        assert_eq!(upper_bits(key, 2), 0b100 << 29);
        assert_eq!(upper_bits(key, 3), 0b1000 << 28);
        assert_eq!(upper_bits(key, 4), 0b10000 << 27);
        assert_eq!(upper_bits(key, 5), 0b100001 << 26);
    }

    #[test]
    fn test_lss_empty() {
        let mut lss: LevelSearch<()> = LevelSearch::new();
        for i in 0..31 {
            if let Entry::Occupied(_) = lss.entry(0, i) {
                panic!("Something was occupied!");
            }
        }
    }

    #[test]
    fn test_lss_insertion_1() {
        let mut lss = LevelSearch::new();
        let ptr = lss.insert(0b11001000001110011111011010100000, b'a');
        assert_eq!(
            unsafe { ptr.as_ref() },
            &LNode {
                key: 0b11001000001110011111011010100000,
                value: b'a',
                prev: ptr::null_mut(),
                next: ptr::null_mut(),
            }
        );

        assert_eq!(lss.root, Some(Descendant::One { min: ptr }));

        let mut map = HashMap::new();
        map.insert(
            0b10000000000000000000000000000000,
            Descendant::One { min: ptr },
        );
        assert_eq!(lss.maps[0], map);

        map.clear();
        map.insert(
            0b11000000000000000000000000000000,
            Descendant::Zero { max: ptr },
        );
        assert_eq!(lss.maps[1], map);

        map.clear();
        map.insert(
            0b11001000001110000000000000000000,
            Descendant::One { min: ptr },
        );
        assert_eq!(lss.maps[14], map);

        map.clear();
        map.insert(
            0b11001000001110011111011010100000,
            Descendant::Zero { max: ptr },
        );
        assert_eq!(lss.maps[30], map);

        unsafe {
            drop(Box::from_raw(ptr.as_ptr()));
        }
    }

    #[test]
    fn test_lss_remove_single_element() {
        let mut lss = LevelSearch::new();
        let p0 = lss.insert(0xF000_0000, b'a');
        lss.remove(unsafe { p0.as_ref() });

        assert_eq!(lss.root, None);
        for map in lss.maps.iter() {
            assert!(map.is_empty());
        }
        unsafe {
            drop(Box::from_raw(p0.as_ptr()));
        }
    }

    #[test]
    fn test_lss_remove_root_both() {
        let mut lss = LevelSearch::new();
        let p0 = lss.insert(0x0000_0000, b'a');
        let p1 = lss.insert(0x8000_0000, b'a');
        assert_eq!(lss.root, Some(Descendant::Both));

        lss.remove(unsafe { p0.as_ref() });
        assert_eq!(lss.root, Some(Descendant::One { min: p1 }));

        let mut expected = HashMap::new();
        expected.insert(0x8000_0000, Descendant::Zero { max: p1 });
        for map in lss.maps.iter() {
            assert_eq!(map, &expected);
        }

        lss.remove(unsafe { p1.as_ref() });
        assert_eq!(lss.root, None);
        for map in lss.maps.iter() {
            assert!(map.is_empty());
        }

        unsafe {
            drop(Box::from_raw(p0.as_ptr()));
            drop(Box::from_raw(p1.as_ptr()));
        }
    }

    #[test]
    fn test_lss_remove_non_root() {
        let mut lss = LevelSearch::new();
        let p0 = lss.insert(0x0000_0000, b'a');
        let p1 = lss.insert(0x0000_F000, b'a');
        lss.remove(unsafe { p1.as_ref() });

        assert_eq!(lss.root, Some(Descendant::Zero { max: p0 }));
        let mut expected = HashMap::new();
        expected.insert(0x0000_0000, Descendant::Zero { max: p0 });
        for map in lss.maps.iter() {
            assert_eq!(map, &expected);
        }

        unsafe {
            drop(Box::from_raw(p0.as_ptr()));
            drop(Box::from_raw(p1.as_ptr()));
        }
    }

    #[test]
    fn test_xfast_integration_insert() {
        let keys: [u32; 32] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7,
        ];

        let mut xfast = XFastMap::new();
        // Empty map, no predecessor
        for key in keys.iter() {
            assert_eq!(xfast.predecessor(*key), None);
        }

        for (i, key) in keys.iter().enumerate() {
            let (key, value) = (*key, i as u32);
            assert_eq!(xfast.insert(key, 0), None);
            // Replace the old key
            assert_eq!(xfast.insert(key, value), Some(0));

            // Predecessor works
            assert_eq!(xfast.predecessor(key), Some(&value));

            // Check that predecessor works for each sorted element
            let mut sorted = (0..=i as u32).collect::<Vec<_>>();
            sorted.sort_unstable_by_key(|k| keys[*k as usize]);
            for (j, ki) in sorted.iter().enumerate() {
                let k = keys[*ki as usize];
                assert_eq!(xfast.predecessor(k), Some(ki));
                assert_eq!(xfast.predecessor(k + 1), Some(ki));

                if j == 0 {
                    assert_eq!(xfast.predecessor(k - 1), None);
                } else {
                    assert_eq!(xfast.predecessor(k - 1), Some(&sorted[j - 1]));
                }
            }
        }
    }

    #[test]
    fn test_xfast_integration_remove() {
        let keys: [u32; 32] = [
            0xcd59c9de, 0x856cb188, 0x6eaaa008, 0xde8db9a9, 0xac3c6ef9,
            0xaba4ba19, 0xc521efbc, 0x866621f3, 0xed3b37a2, 0xda2a7ce7,
            0x63df9f0a, 0xb2e4be7c, 0x9c69cb0d, 0x808375c4, 0xbc42de68,
            0x73f9c015, 0x72903697, 0xb12ad490, 0x9282c1c2, 0x8d4ac30e,
            0xfb1c49e7, 0x9ffdd800, 0x40fd421f, 0x3aa9e7b1, 0x7a20774e,
            0xb940e532, 0x749fee0d, 0x0e6c8517, 0x0fa4dc69, 0x205ec45f,
            0xc8281c71, 0xedd6b0c7,
        ];
        let mut xfast = XFastMap::new();
        for (i, key) in keys.iter().enumerate() {
            let (key, value) = (*key, i as u32);
            assert_eq!(xfast.insert(key, value), None);
        }

        for (i, key) in keys.iter().enumerate() {
            let (key, value) = (*key, i as u32);
            assert_eq!(xfast.remove(key), Some(value));
            assert_eq!(xfast.remove(key), None);

            // Check that predecessor works for each sorted element
            let mut sorted = (1 + i as u32..32).collect::<Vec<_>>();
            sorted.sort_unstable_by_key(|k| keys[*k as usize]);
            for (j, ki) in sorted.iter().enumerate() {
                let k = keys[*ki as usize];
                assert_eq!(xfast.predecessor(k), Some(ki));
                assert_eq!(xfast.predecessor(k + 1), Some(ki));

                if j == 0 {
                    assert_eq!(xfast.predecessor(k - 1), None);
                } else {
                    assert_eq!(xfast.predecessor(k - 1), Some(&sorted[j - 1]));
                }
            }
        }
    }
}
