use std::iter::FromIterator;

use bit_parallel::u32x16;

use super::{Bits512, SelectRank};
use crate::tree::{PackedPtr, Ptr, PtrMut};

const CAPACITY: usize = 16;

pub struct BitVec {
    root: Box<Node>,
}

impl BitVec {
    pub fn new() -> BitVec {
        BitVec {
            root: Box::new(Node {
                lens: [0; CAPACITY],
                n_ones: [0; CAPACITY],
                ptrs: [PackedPtr::null(); CAPACITY],
            }),
        }
    }

    pub fn total_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.root.total_size()
    }

    pub fn len(&self) -> usize {
        self.root.lens[CAPACITY - 1] as usize
    }

    pub fn num_ones(&self) -> u32 {
        self.root.n_ones[CAPACITY - 1]
    }

    pub fn num_zeros(&self) -> u32 {
        self.len() as u32 - self.num_ones()
    }

    fn split(&mut self, stack: Vec<(*mut Node, usize)>, new: Box<Bits512>) {
        let mut ptr = PackedPtr::from_leaf(new);
        for (node, rank) in stack.iter().rev().cloned() {
            let node = unsafe { &mut *node };
            if !node.is_full() {
                node.insert(rank, ptr);
                node.lens[rank] -= ptr.len() as u32;
                node.n_ones[rank] -= ptr.num_ones();
                return;
            } else {
                let mut new = Box::new(node.split());
                if rank >= 8 {
                    new.insert(rank - 8, ptr);
                    new.lens[rank - 8] -= ptr.len() as u32;
                    new.n_ones[rank - 8] -= ptr.num_ones();
                } else {
                    node.insert(rank, ptr);
                    node.lens[rank] -= ptr.len() as u32;
                    node.n_ones[rank] -= ptr.num_ones();
                }
                ptr = PackedPtr::from_inner(new);
            }
        }

        // We've recursed all the way to the root
        debug_assert!(!self.root.is_full());
        debug_assert!(self.root.ptrs[9].is_null());

        let len = self.root.len() + ptr.len();
        let n_ones = self.root.num_ones() + ptr.num_ones();

        let root = std::mem::replace(
            &mut self.root,
            Box::new(Node {
                lens: [len as u32; CAPACITY],
                n_ones: [n_ones; CAPACITY],
                ptrs: [PackedPtr::null(); CAPACITY],
            }),
        );
        self.root.lens[0] = root.len() as u32;
        self.root.n_ones[0] = root.num_ones() as u32;
        self.root.ptrs[0] = PackedPtr::from_inner(root);
        self.root.ptrs[1] = ptr;
    }

    fn approx_depth(&self) -> usize {
        // leading zeros is approximately log2. Divide by 4 to get log_16
        (64 - self.len().leading_zeros() as usize) / 4
    }

    pub fn insert(&mut self, index: usize, bit: bool) {
        debug_assert!(index <= self.len());
        if index == 0 && self.len() == 0 {
            self.root.ptrs[0] =
                PackedPtr::from_leaf(Box::new(Bits512::from(bit)));
            self.root.lens = [1; CAPACITY];
            self.root.n_ones = [bit as u32; CAPACITY];
            return;
        }

        let mut index = index as u32;

        let mut stack: Vec<(*mut Node, usize)> =
            Vec::with_capacity(self.approx_depth());
        let mut node: &mut Node = &mut self.root;

        loop {
            let rank = u32x16::rank(&node.lens, index) as usize;
            if rank > 0 {
                index -= node.lens[rank - 1];
            }
            node.add_bit_count(rank, bit);

            // Use an unsafe *mut raw pointer to work around borrow checker
            // restrictions (we are "releasing" the earlier borrows when
            // we reassign node, so there is never a double mutable borrow)
            let n = &mut node.ptrs[rank] as *mut PackedPtr<Node, Bits512>;
            match unsafe { &mut *n }.expand_mut() {
                PtrMut::None => unreachable!(),
                PtrMut::Inner(inner) => {
                    stack.push((node as *mut _, rank));
                    node = inner;
                }
                PtrMut::Leaf(leaf) => {
                    if leaf.is_full() {
                        let mut new = Box::new(leaf.split());
                        if index >= 256 {
                            new.insert(index as usize - 256, bit);
                        } else {
                            leaf.insert(index as usize, bit);
                        }
                        stack.push((node, rank));
                        self.split(stack, new);
                    } else {
                        leaf.insert(index as usize, bit);
                    }
                    break;
                }
            }
        }
    }

    pub fn set_bit(&mut self, index: usize, bit: bool) {
        let mut index = index as u32;
        let mut stack: Vec<(*mut Node, usize)> =
            Vec::with_capacity((64 - self.len().leading_zeros() as usize) / 4);
        let mut node: &mut Node = &mut self.root;

        let old_bit = loop {
            let rank = u32x16::rank(&node.lens, 1 + index) as usize;
            if rank > 0 {
                index -= node.lens[rank - 1];
            }
            stack.push((node as *mut _, rank));

            // Use an unsafe *mut raw pointer to work around borrow checker
            // restrictions (we are "releasing" the earlier borrows when
            // we reassign node, so there is never a double mutable borrow)
            let n = &mut node.ptrs[rank] as *mut PackedPtr<Node, Bits512>;
            match unsafe { &mut *n }.expand_mut() {
                PtrMut::None => unreachable!(),
                PtrMut::Inner(inner) => {
                    node = inner;
                }
                PtrMut::Leaf(leaf) => {
                    let old_bit = leaf.get_bit(index as usize);
                    leaf.set_bit(index as usize, bit);
                    break old_bit;
                }
            }
        };

        if !old_bit && bit {
            for (node, rank) in stack.iter().cloned() {
                let node = unsafe { node.as_mut().unwrap() };
                u32x16::increment(&mut node.n_ones, rank);
            }
        } else if old_bit && !bit {
            for (node, rank) in stack.iter().cloned() {
                let node = unsafe { node.as_mut().unwrap() };
                u32x16::decrement(&mut node.n_ones, rank);
            }
        }
    }

    fn iter_leaf(&self) -> impl Iterator<Item = &Bits512> {
        let mut current: &Node = &self.root;
        let mut stack = Vec::with_capacity(self.approx_depth());
        while let Ptr::Inner(next) = current.ptrs[0].expand() {
            stack.push((current, 1usize));
            current = next;
        }
        stack.push((current, 0usize));
        std::iter::from_fn(move || {
            while let Some((current, pos)) = stack.pop() {
                match current.ptrs[pos].expand() {
                    Ptr::None => {
                        for i in pos..current.ptrs.len() {
                            debug_assert!(current.ptrs[i].is_null());
                        }
                        continue;
                    }
                    Ptr::Leaf(leaf) => {
                        if pos + 1 < current.ptrs.len() {
                            stack.push((current, pos + 1));
                        }
                        return Some(leaf);
                    }
                    Ptr::Inner(mut inner) => {
                        if pos + 1 < current.ptrs.len() {
                            stack.push((current, pos + 1));
                        }
                        while let Ptr::Inner(new) = inner.ptrs[0].expand() {
                            stack.push((inner, 1usize));
                            inner = new;
                        }

                        if let Ptr::Leaf(leaf) = inner.ptrs[0].expand() {
                            stack.push((inner, 1usize));
                            return Some(leaf);
                        }
                    }
                }
            }
            None
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        self.iter_leaf().flat_map(|bits| bits.iter())
    }
}

impl FromIterator<bool> for BitVec {
    fn from_iter<T>(input: T) -> Self
    where
        T: IntoIterator<Item = bool>,
    {
        let mut bits = BitVec::new();
        for bit in input.into_iter() {
            bits.insert(bits.len(), bit);
        }
        bits
    }
}

impl SelectRank for BitVec {
    fn get_bit(&self, mut index: usize) -> bool {
        debug_assert!(index < self.root.lens[CAPACITY - 1] as usize);
        let mut node: &Node = &self.root;

        loop {
            // Add one because we are 0-indexed
            let rank = u32x16::rank(&node.lens, 1 + index as u32) as usize;
            if rank > 0 {
                index -= node.lens[rank - 1] as usize;
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return leaf.get_bit(index);
                }
            }
        }
    }

    /// Return the position of the `i`th 0 (0-indexed)
    fn select0(&self, index: usize) -> usize {
        debug_assert!(
            index
                < (self.root.lens[CAPACITY - 1]
                    - self.root.n_ones[CAPACITY - 1])
                    as usize
        );

        let mut index = index as u32;

        let mut node: &Node = &self.root;
        let mut count = 0;

        loop {
            let rank =
                u32x16::rank_diff(&node.lens, &node.n_ones, index + 1) as usize;
            if rank > 0 {
                count += node.lens[rank - 1];
                index -= node.lens[rank - 1] - node.n_ones[rank - 1];
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return count as usize + leaf.select0(index as usize);
                }
            }
        }
    }
    /// Return the position of the `i`th 1 (0-indexed)
    fn select1(&self, mut index: usize) -> usize {
        debug_assert!(index < self.root.n_ones[CAPACITY - 1] as usize);
        let mut node: &Node = &self.root;
        let mut count = 0;

        loop {
            // Add one because we are 0-indexed
            let rank = u32x16::rank(&node.n_ones, 1 + index as u32) as usize;
            if rank > 0 {
                count += node.lens[rank - 1];
                index -= node.n_ones[rank - 1] as usize;
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return count as usize + leaf.select1(index);
                }
            }
        }
    }

    /// Return the number of 0s before the `i`th position
    fn rank0(&self, index: usize) -> usize {
        index - self.rank1(index)
    }

    /// Return the number of 1s before the `i`th position
    fn rank1(&self, index: usize) -> usize {
        debug_assert!(index < self.len());
        let mut index = index as u32;

        let mut node: &Node = &self.root;
        let mut count = 0;
        loop {
            // Add one because we are 0-indexed
            let rank = u32x16::rank(&node.lens, index + 1) as usize;
            if rank > 0 {
                count += node.n_ones[rank - 1];
                index -= node.lens[rank - 1];
            }

            match node.ptrs[rank].expand() {
                Ptr::None => unreachable!(),
                Ptr::Inner(inner) => {
                    node = inner;
                }
                Ptr::Leaf(leaf) => {
                    return count as usize + leaf.rank1(index as usize);
                }
            }
        }
    }
}

#[derive(Default, Debug, Eq, PartialEq)]
struct Node {
    lens: [u32; CAPACITY],
    n_ones: [u32; CAPACITY],
    ptrs: [PackedPtr<Node, Bits512>; CAPACITY],
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
    fn total_size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        for ptr in self.ptrs.iter() {
            match ptr.expand() {
                Ptr::None => break,
                Ptr::Leaf(leaf) => size += std::mem::size_of_val(leaf),
                Ptr::Inner(inner) => size += inner.total_size(),
            }
        }

        size
    }

    fn split(&mut self) -> Node {
        debug_assert!(!self.ptrs[CAPACITY - 1].is_null());
        debug_assert!(self.lens[CAPACITY - 1] > self.lens[CAPACITY - 2]);

        let mut node = Node {
            lens: [0; CAPACITY],
            n_ones: [0; CAPACITY],
            ptrs: [PackedPtr::null(); CAPACITY],
        };

        u32x16::split(&mut self.lens, &mut node.lens);
        u32x16::split(&mut self.n_ones, &mut node.n_ones);

        self.ptrs[8..].swap_with_slice(&mut node.ptrs[..8]);
        node
    }

    fn insert(&mut self, rank: usize, ptr: PackedPtr<Node, Bits512>) {
        debug_assert!(rank < CAPACITY - 1);
        debug_assert!(self.ptrs[CAPACITY - 1].is_null());
        debug_assert!(!self.ptrs[rank].is_null());
        unsafe {
            std::ptr::copy(
                &self.ptrs[rank] as *const _,
                &mut self.ptrs[rank + 1] as *mut _,
                CAPACITY - 1 - rank,
            );
            std::ptr::copy(
                &self.lens[rank] as *const _,
                &mut self.lens[rank + 1] as *mut _,
                CAPACITY - 1 - rank,
            );
            std::ptr::copy(
                &self.n_ones[rank] as *const _,
                &mut self.n_ones[rank + 1] as *mut _,
                CAPACITY - 1 - rank,
            );
        }
        self.ptrs[rank + 1] = ptr;
    }

    fn add_bit_count(&mut self, rank: usize, bit: bool) {
        u32x16::increment(&mut self.lens, rank);
        if bit {
            u32x16::increment(&mut self.n_ones, rank);
        }
    }

    fn is_full(&self) -> bool {
        debug_assert_eq!(
            self.ptrs[CAPACITY - 1].is_null(),
            self.lens[CAPACITY - 1] == self.lens[CAPACITY - 2]
        );
        !self.ptrs[CAPACITY - 1].is_null()
    }

    fn num_ones(&self) -> u32 {
        self.n_ones[CAPACITY - 1] as u32
    }

    fn len(&self) -> usize {
        self.lens[CAPACITY - 1] as usize
    }

    #[cfg(test)]
    fn debug_assert_indices(&self) {
        let mut len = 0;
        let mut n_ones = 0;

        for (i, ptr) in self.ptrs.iter().enumerate() {
            match ptr.expand() {
                Ptr::None => {}
                Ptr::Leaf(l) => {
                    len += l.len() as u32;
                    n_ones += l.num_ones();
                }
                Ptr::Inner(i) => {
                    len += i.len() as u32;
                    n_ones += i.num_ones();
                }
            }

            if i < CAPACITY {
                debug_assert_eq!(len, self.lens[i]);
                debug_assert_eq!(n_ones, self.n_ones[i]);
            }
        }
    }
}

impl PackedPtr<Node, Bits512> {
    fn num_ones(self) -> u32 {
        debug_assert!(!self.is_null());
        match self.expand() {
            Ptr::None => unreachable!(),
            Ptr::Leaf(leaf) => leaf.num_ones(),
            Ptr::Inner(inner) => inner.num_ones(),
        }
    }

    fn len(self) -> usize {
        match self.expand() {
            Ptr::None => unreachable!(),
            Ptr::Leaf(leaf) => leaf.len(),
            Ptr::Inner(inner) => inner.len(),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use bit_parallel::u9x7::u9x7;

    #[test]
    fn test_bitvec_total_size() {
        // In the worst case (assuming that all nodes in our tree are at
        // the minimum size), we have:
        //  - Bits512: 640 bits / 256                 = 2.5 n
        //  - Tree: \sum 1/8^k * 2048 bits / 8 blocks
        //      = 8/7 * 2048 / (8 * 256)
        //      = 1.14
        // which should be about 3.64x bits to store in the worst case

        let bits = BitVec::from_iter(vec![true; 80]);
        // 304 bytes to store 10 bytes of data
        //  = 34.4 x overhead
        assert_eq!(bits.total_size(), 344);

        let bits = BitVec::from_iter(vec![false; 80000]);
        // 47240 bytes to store 10000 bytes of data
        //  = 3.60x overhead
        assert_eq!(bits.total_size(), 35976);
    }

    #[test]
    fn test_bitvec_insert_bit() {
        let mut bits = BitVec::new();

        bits.insert(0, true);
        assert_eq!(bits.root.n_ones, [1; CAPACITY]);
        assert_eq!(bits.root.lens, [1; CAPACITY]);
        assert_eq!(bits.len(), 1);
        assert_eq!(bits.num_zeros(), 0);
        assert_eq!(bits.num_ones(), 1);

        bits.insert(0, false);
        assert_eq!(bits.root.n_ones, [1; CAPACITY]);
        assert_eq!(bits.root.lens, [2; CAPACITY]);
        assert_eq!(bits.len(), 2);
        assert_eq!(bits.num_zeros(), 1);
        assert_eq!(bits.num_ones(), 1);

        bits.insert(2, true);
        assert_eq!(bits.root.n_ones, [2; CAPACITY]);
        assert_eq!(bits.root.lens, [3; CAPACITY]);
        assert_eq!(bits.len(), 3);
        assert_eq!(bits.num_zeros(), 1);
        assert_eq!(bits.num_ones(), 2);
    }

    #[test]
    fn test_bitvec_single_level_full_zeros() {
        let mut bits = BitVec::new();
        for _ in 0..(256 + 256 * CAPACITY) {
            bits.insert(0, false);

            for i in 0..CAPACITY {
                assert!(bits.root.lens[i] <= 512 * i as u32 + 512);
                assert_eq!(bits.root.n_ones[i], 0);

                if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                    assert_eq!(leaf.num_ones(), 0);
                }
            }
        }

        let mut len = 0;
        for i in 0..CAPACITY {
            assert!(bits.root.lens[i] <= 512 * i as u32 + 512);
            if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                len += leaf.len();
                assert_eq!(bits.root.lens[i] as usize, len);
                assert_eq!(leaf.num_ones(), 0);
                assert_eq!(bits.root.n_ones[i], 0);
            } else {
                unreachable!();
            }
        }

        for i in 0..(256 + 256 * CAPACITY) {
            assert_eq!(bits.rank0(i), i);
            assert_eq!(bits.rank1(i), 0);
            assert_eq!(bits.select0(i), i);
        }
    }

    #[test]
    fn test_bitvec_single_level_full_ones() {
        let mut bits = BitVec::new();
        for i in 0..(256 + 256 * CAPACITY) {
            bits.insert(i, true);

            for j in 0..CAPACITY {
                assert!(bits.root.lens[j] <= 512 * j as u32 + 512);
            }
        }

        let mut len = 0;
        let mut n_ones = 0;
        for i in 0..CAPACITY {
            assert!(bits.root.lens[i] <= 512 * i as u32 + 512);
            if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                len += leaf.len();
                n_ones += leaf.num_ones();
                assert_eq!(bits.root.lens[i] as usize, len);
                assert_eq!(bits.root.n_ones[i], n_ones);
            } else {
                unreachable!();
            }
        }

        for i in 0..(256 + 256 * CAPACITY) {
            assert_eq!(bits.rank0(i), 0);
            assert_eq!(bits.rank1(i), i);
            assert_eq!(bits.select1(i), i);
        }
    }

    #[test]
    fn test_bitvec_single_level_half_zeros() {
        let mut bits = BitVec::new();
        let mut expected = Vec::with_capacity(256 + 256 * CAPACITY);
        for i in 0..(128 + 128 * CAPACITY) {
            bits.insert(i, true);
            bits.insert(i, false);

            expected.insert(i, true);
            expected.insert(i, false);

            assert_eq!(expected, bits.iter().collect::<Vec<_>>());

            for j in 0..CAPACITY {
                assert!(bits.root.lens[j] <= 512 * j as u32 + 512);
            }
        }
        // bits should be a palindrome of 0^k 1^k

        let mut len = 0;
        let mut n_ones = 0;
        for i in 0..CAPACITY {
            assert!(bits.root.lens[i] <= 512 * i as u32 + 512);
            if let Ptr::Leaf(leaf) = bits.root.ptrs[i].expand() {
                len += leaf.len();
                n_ones += leaf.num_ones();
                assert_eq!(bits.root.lens[i] as usize, len);
                assert_eq!(bits.root.n_ones[i], n_ones);
            } else {
                unreachable!();
            }
        }

        let len = 128 + 128 * CAPACITY;
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

    #[test]
    fn test_bitvec_set_bit() {
        let mut bits = BitVec::from_iter(vec![false; 1000]);
        let mut expected = Vec::with_capacity(1000);
        for i in 0..1000 {
            bits.get_bit(i);
            bits.set_bit(i, i % 2 == 0);

            expected.push(i % 2 == 0);
            bits.root.debug_assert_indices();
        }

        assert_eq!(bits.iter().collect::<Vec<_>>(), expected);
        bits.root.debug_assert_indices();
    }

    #[test]
    fn test_node_split_1() {
        let mut node = Node {
            lens: [512; CAPACITY],
            n_ones: [512; CAPACITY],
            ptrs: [PackedPtr::null(); CAPACITY],
        };

        let ptrs = (0..CAPACITY)
            .map(|_| {
                PackedPtr::from_leaf(Box::new(Bits512 {
                    n_ones: u9x7::new([64, 128, 192, 256, 320, 384, 448]),
                    len: 512,
                    bits: [u64::max_value(); 8],
                }))
            })
            .collect::<Vec<_>>();
        for i in 0..CAPACITY {
            node.ptrs[i] = ptrs[i];
            node.lens[i] = 512 + i as u32 * 512;
            node.n_ones[i] = 512 + i as u32 * 512;
        }

        let new = node.split();
        let expected = [
            512, 1024, 1536, 2048, 2560, 3072, 3584, 4096, 4096, 4096, 4096,
            4096, 4096, 4096, 4096, 4096,
        ];
        debug_assert_eq!(new.lens, expected);
        debug_assert_eq!(node.lens, expected);

        debug_assert_eq!(new.n_ones, expected);
        debug_assert_eq!(node.n_ones, expected);

        let null = PackedPtr::null();
        debug_assert_eq!(
            node.ptrs,
            [
                ptrs[0], ptrs[1], ptrs[2], ptrs[3], ptrs[4], ptrs[5], ptrs[6],
                ptrs[7], null, null, null, null, null, null, null, null
            ]
        );
        debug_assert_eq!(
            new.ptrs,
            [
                ptrs[8], ptrs[9], ptrs[10], ptrs[11], ptrs[12], ptrs[13],
                ptrs[14], ptrs[15], null, null, null, null, null, null, null,
                null,
            ]
        );
    }

    #[test]
    fn test_bitvec_multilevel_palindrome() {
        assert!(2 * 5000 > CAPACITY * 512);
        let mut bits = BitVec::new();
        let mut expected = Vec::with_capacity(10000);
        for i in 0..5000 {
            bits.insert(i, true);
            expected.insert(i, true);
            assert_eq!(expected, bits.iter().collect::<Vec<_>>());

            bits.insert(i, false);
            expected.insert(i, false);
            assert_eq!(expected.len(), bits.len());
            assert_eq!(expected, bits.iter().collect::<Vec<_>>());
        }
        // bits should be a palindrome of 0^k 1^k

        for i in 0..5000 {
            assert_eq!(bits.rank0(i), i);
            assert_eq!(bits.rank1(i), 0);
            assert_eq!(bits.get_bit(i), expected[i]);
        }
        for i in 5000..10000 {
            assert_eq!(bits.rank0(i), 5000);
            assert_eq!(bits.rank1(i), i - 5000);
            assert_eq!(bits.get_bit(i), expected[i]);
        }
        for i in 0..5000 {
            assert_eq!(bits.select0(i), i);
            assert_eq!(bits.select1(i), 5000 + i);
        }
    }
}
