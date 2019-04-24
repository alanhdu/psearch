#![allow(dead_code)]
use super::array;
use super::Bits256;

const B: usize = 8;
const MIN_SIZE: usize = B - 1;
const CAPACITY: usize = 16;

pub struct BitVec {
    root: Node,
}

impl BitVec {
    pub fn new() -> BitVec {
        BitVec {
            root: Node {
                counts: [0; 16],
                ones: [0; 16],
                ptrs: [PackedPtr::null(); CAPACITY],
            },
        }
    }

    pub fn len(&self) -> usize {
        self.root.counts[15] as usize
    }

    pub fn num_ones(&self) -> u32 {
        self.root.ones[15]
    }

    pub fn num_zeros(&self) -> u32 {
        self.len() as u32 - self.num_ones()
    }

    pub fn insert(&mut self, index: usize, bit: bool) {
        debug_assert!(index <= self.len());
        if index == 0 && self.len() == 0 {
            self.root.ptrs[0] = PackedPtr::from(Box::new(Bits256::from(bit)));
            self.root.counts = [1; 16];
            self.root.ones = [bit as u32; 16];
            return;
        }

        let mut index = index as u32;

        let mut stack = Vec::new();
        let mut node = &mut self.root;
        let mut rank = array::rank(&node.counts, index) as usize;

        loop {
            if rank > 0 {
                index -= node.counts[rank - 1];
            }

            array::increment(&mut node.counts, rank);
            if bit {
                array::increment(&mut node.ones, rank);
            }

            // Use an unsafe *mut raw pointer to work around borrow checker
            // restrictions (we are "releasing" the earlier borrows when
            // we reassign node, so there is never a double mutable borrow)
            let n = &mut node.ptrs[rank] as *mut PackedPtr;
            match unsafe { &mut *n }.expand_mut() {
                PtrMut::None => unreachable!(),
                PtrMut::Inner(inner) => {
                    stack.push((node as *mut _, rank));
                    node = inner;
                    rank = array::rank(&node.counts, index) as usize;
                }
                PtrMut::Leaf(leaf) => {
                    if leaf.is_full() {
                        let mut new = Box::new(leaf.split());
                        if node.counts[15] == node.counts[14] {
                            // We have space!
                            debug_assert!(rank < 15);
                            debug_assert!(node.ptrs[15].is_null());
                            unsafe {
                                std::ptr::copy(
                                    &node.ptrs[rank] as *const _,
                                    &mut node.ptrs[rank + 1] as *mut _,
                                    15 - rank,
                                );
                                std::ptr::copy(
                                    &node.counts[rank] as *const _,
                                    &mut node.counts[rank + 1] as *mut _,
                                    15 - rank,
                                );
                                std::ptr::copy(
                                    &node.ones[rank] as *const _,
                                    &mut node.ones[rank + 1] as *mut _,
                                    15 - rank,
                                );
                            }
                            if index >= 128 {
                                new.insert_bit(index as usize - 128, bit);
                            } else {
                                leaf.insert_bit(index as usize, bit);
                            }

                            node.counts[rank] -= 128 + (index >= 128) as u32;
                            node.ones[rank] -= new.num_ones();

                            node.ptrs[rank + 1] = PackedPtr::from(new);
                        } else {
                            // TODO: recurse upwards
                            unimplemented!();
                        }
                    } else {
                        leaf.insert_bit(index as usize, bit);
                    }
                    break;
                }
            }
        }
    }

    /// Return the position of the `i`th 0 (0-indexed)
    pub fn select0(&self, mut index: u32) -> u32 {
        debug_assert!(index < self.root.counts[15] - self.root.ones[15]);

        let mut node = &self.root;
        let mut count = 0;

        loop {
            let rank =
                array::rank_diff(&node.counts, &node.ones, index + 1) as usize;
            if rank > 0 {
                count += node.counts[rank - 1];
                index -= node.counts[rank - 1] - node.ones[rank - 1];
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return count + leaf.select0(index);
                }
            }
        }
    }
    /// Return the position of the `i`th 1 (0-indexed)
    pub fn select1(&self, mut index: u32) -> u32 {
        debug_assert!(index < self.root.ones[15]);
        let mut node = &self.root;
        let mut count = 0;

        loop {
            let rank = array::rank(&node.ones, index + 1) as usize;
            if rank > 0 {
                count += node.counts[rank - 1];
                index -= node.ones[rank - 1];
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return count + leaf.select1(index);
                }
            }
        }
    }

    /// Return the number of 0s before the `i`th position
    pub fn rank0(&self, index: u32) -> u32 {
        index - self.rank1(index)
    }

    /// Return the number of 1s before the `i`th position
    pub fn rank1(&self, mut index: u32) -> u32 {
        debug_assert!((index as usize) < self.len());

        let mut node = &self.root;
        let mut count = 0;
        loop {
            let rank = array::rank(&node.counts, index + 1) as usize;
            if rank > 0 {
                count += node.ones[rank - 1];
                index -= node.counts[rank - 1];
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return count + leaf.rank1(index);
                }
            }
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
struct Node {
    counts: [u32; 16],
    ones: [u32; 16],
    ptrs: [PackedPtr; CAPACITY],
}

impl Drop for Node {
    fn drop(&mut self) {
        for ptr in self.ptrs.iter_mut() {
            match ptr.expand_mut() {
                PtrMut::None => {}
                PtrMut::Leaf(leaf) => unsafe {
                    drop(Box::from_raw(leaf as *mut _));
                },
                PtrMut::Inner(inner) => unsafe {
                    drop(Box::from_raw(inner as *mut _));
                },
            }
        }
    }
}

impl Node {
    fn split(&mut self) -> Node {
        debug_assert!(!self.ptrs[15].is_null());
        debug_assert!(self.counts[15] > self.counts[14]);

        let mut node = Node {
            counts: [0; 16],
            ones: [0; 16],
            ptrs: [PackedPtr::null(); CAPACITY],
        };

        unsafe {
            std::ptr::copy_nonoverlapping(
                &self.ptrs[8] as *const _,
                &mut node.ptrs[0] as *mut _,
                8,
            );
        }

        for i in 8..16 {
            node.counts[i] = self.counts[i] - self.counts[7];
            self.counts[i] = self.counts[7];
            node.ones[i] = self.ones[i] - self.ones[7];
            self.ones[i] = self.ones[7];
        }
        node
    }

    fn num_ones(&self) -> u32 {
        self.ones[15] as u32
    }

    fn len(&self) -> usize {
        self.counts[15] as usize
    }

    #[cfg(test)]
    fn to_vec(&self) -> Vec<bool> {
        let mut vec = Vec::with_capacity(self.counts[15] as usize);
        for ptr in &self.ptrs {
            match ptr.expand() {
                Ptr::None => {}
                Ptr::Leaf(l) => {
                    vec.append(&mut l.to_vec());
                }
                Ptr::Inner(inner) => {
                    vec.append(&mut inner.to_vec());
                }
            }
        }
        vec
    }
}

#[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
struct PackedPtr(usize);

enum Ptr<'a> {
    None,
    Leaf(&'a Bits256),
    Inner(&'a Node),
}

enum PtrMut<'a> {
    None,
    Leaf(&'a mut Bits256),
    Inner(&'a mut Node),
}

impl PackedPtr {
    fn null() -> PackedPtr {
        PackedPtr(0)
    }

    fn is_null(self) -> bool {
        self.0 == 0
    }

    fn is_full(self) -> bool {
        match self.expand() {
            Ptr::None => false,
            Ptr::Leaf(leaf) => leaf.is_full(),
            Ptr::Inner(node) => !node.ptrs[15].is_null(),
        }
    }

    fn split(&mut self) -> PackedPtr {
        debug_assert!(self.is_full());

        match self.expand_mut() {
            PtrMut::None => unreachable!(),
            PtrMut::Leaf(leaf) => PackedPtr::from(Box::new(leaf.split())),
            PtrMut::Inner(inner) => PackedPtr::from(Box::new(inner.split())),
        }
    }

    fn num_ones(self) -> u32 {
        debug_assert!(!self.is_null());
        match self.expand() {
            Ptr::None => unreachable!(),
            Ptr::Leaf(leaf) => leaf.num_ones(),
            Ptr::Inner(inner) => inner.num_ones(),
        }
    }

    fn len(self) -> usize {
        debug_assert!(!self.is_null());
        match self.expand() {
            Ptr::None => unreachable!(),
            Ptr::Leaf(leaf) => leaf.len(),
            Ptr::Inner(inner) => inner.len(),
        }
    }

    fn expand(&self) -> Ptr<'_> {
        if self.is_null() {
            Ptr::None
        } else if self.0 & 0b1 == 0 {
            Ptr::Leaf(unsafe { &*(self.0 as *const _) })
        } else {
            Ptr::Inner(unsafe { &*((self.0 - 1) as *const _) })
        }
    }

    fn expand_mut(&mut self) -> PtrMut<'_> {
        if self.is_null() {
            PtrMut::None
        } else if self.0 & 0b1 == 0 {
            PtrMut::Leaf(unsafe { &mut *(self.0 as *mut _) })
        } else {
            PtrMut::Inner(unsafe { &mut *((self.0 - 1) as *mut _) })
        }
    }
}

impl From<Box<Node>> for PackedPtr {
    fn from(node: Box<Node>) -> PackedPtr {
        let value = Box::into_raw(node) as usize;
        debug_assert_eq!(value & 0b1, 0);
        PackedPtr(value | 1)
    }
}

impl From<Box<Bits256>> for PackedPtr {
    fn from(node: Box<Bits256>) -> PackedPtr {
        let value = Box::into_raw(node) as usize;
        debug_assert_eq!(value & 0b1, 0);
        PackedPtr(value)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bitvec_insert_bit() {
        let mut bits = BitVec::new();

        bits.insert(0, true);
        assert_eq!(bits.root.ones, [1; 16]);
        assert_eq!(bits.root.counts, [1; 16]);
        assert_eq!(bits.len(), 1);
        assert_eq!(bits.num_zeros(), 0);
        assert_eq!(bits.num_ones(), 1);

        bits.insert(0, false);
        assert_eq!(bits.root.ones, [1; 16]);
        assert_eq!(bits.root.counts, [2; 16]);
        assert_eq!(bits.len(), 2);
        assert_eq!(bits.num_zeros(), 1);
        assert_eq!(bits.num_ones(), 1);

        bits.insert(2, true);
        assert_eq!(bits.root.ones, [2; 16]);
        assert_eq!(bits.root.counts, [3; 16]);
        assert_eq!(bits.len(), 3);
        assert_eq!(bits.num_zeros(), 1);
        assert_eq!(bits.num_ones(), 2);
    }

    #[test]
    fn test_bitvec_single_level_full_zeros() {
        let mut bits = BitVec::new();
        for _ in 0..(128 + 128 * CAPACITY) {
            bits.insert(0, false);

            for i in 0..16 {
                assert!(bits.root.counts[i] <= 256 * i as u32 + 256);
            }
        }

        let mut len = 0;
        let mut n_ones = 0;
        for i in 0..16 {
            assert!(bits.root.counts[i] <= 256 * i as u32 + 256);
            if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                len += leaf.len();
                n_ones += leaf.num_ones();
                assert_eq!(bits.root.counts[i] as usize, len);
                assert_eq!(bits.root.ones[i], n_ones);
            } else {
                unreachable!();
            }
        }

        for i in 0..(128 + 128 * CAPACITY as u32) {
            assert_eq!(bits.rank0(i), i);
            assert_eq!(bits.rank1(i), 0);
            assert_eq!(bits.select0(i), i);
        }
    }

    #[test]
    fn test_bitvec_single_level_full_ones() {
        let mut bits = BitVec::new();
        for i in 0..(128 + 128 * CAPACITY) {
            bits.insert(i, true);

            for j in 0..16 {
                assert!(bits.root.counts[j] <= 256 * j as u32 + 256);
            }
        }

        let mut len = 0;
        let mut n_ones = 0;
        for i in 0..16 {
            assert!(bits.root.counts[i] <= 256 * i as u32 + 256);
            if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                len += leaf.len();
                n_ones += leaf.num_ones();
                assert_eq!(bits.root.counts[i] as usize, len);
                assert_eq!(bits.root.ones[i], n_ones);
            } else {
                unreachable!();
            }
        }

        for i in 0..(128 + 128 * CAPACITY as u32) {
            assert_eq!(bits.rank0(i), 0);
            assert_eq!(bits.rank1(i), i);
            assert_eq!(bits.select1(i), i);
        }
    }

    #[test]
    fn test_bitvec_single_level_half_zeros() {
        let mut bits = BitVec::new();
        let mut expected = Vec::with_capacity(128 + 128 * CAPACITY);
        for i in 0..(64 + 64 * CAPACITY) {
            bits.insert(i, true);
            bits.insert(i, false);

            expected.insert(i, true);
            expected.insert(i, false);

            assert_eq!(expected, bits.root.to_vec());

            for j in 0..16 {
                assert!(bits.root.counts[j] <= 256 * j as u32 + 256);
            }
        }
        // bits should be a palindrome of 0^k 1^k

        let mut len = 0;
        let mut n_ones = 0;
        for i in 0..16 {
            assert!(bits.root.counts[i] <= 256 * i as u32 + 256);
            if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                len += leaf.len();
                n_ones += leaf.num_ones();
                assert_eq!(bits.root.counts[i] as usize, len);
                assert_eq!(bits.root.ones[i], n_ones);
            } else {
                unreachable!();
            }
        }

        let len = 64 + 64 * CAPACITY as u32;
        for i in 0..(2 * len) {
            if i < len {
                assert_eq!(bits.rank0(i), i);
                assert_eq!(bits.rank1(i), 0)
            } else {
                assert_eq!(bits.rank0(i), len);
                assert_eq!(bits.rank1(i), i - len);
            }
        }

        for i in 0..len {
            assert_eq!(bits.select0(i), i);
            assert_eq!(bits.select1(i), len + i);
        }
    }
}
