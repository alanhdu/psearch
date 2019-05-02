use crate::tree::{Leaf, Tree};
use crate::utils::binary_search_rank;

const CAPACITY: usize = 16;

pub(super) type ByteTree = Tree<ByteLeaf>;

impl ByteTree {
    pub(crate) fn child_number(
        &self,
        index: usize,
        degree: usize,
        needle: u8,
    ) -> (usize, u8) {
        let (leaf, index) = self.get_leaf(index);
        leaf.child_number(index, degree, needle)
    }

    pub(crate) fn get(&self, index: usize) -> u8 {
        let (leaf, index) = self.get_leaf(index);
        leaf.bytes[index]
    }

    #[cfg(test)]
    pub(crate) fn to_vec(&self) -> Vec<u8> {
        (0..self.len()).map(|i| self.get(i)).collect::<Vec<_>>()
    }
}

pub(super) struct ByteLeaf {
    len: u8,
    bytes: [u8; 255],
    next: *mut ByteLeaf,
}

impl Leaf for ByteLeaf {
    type Output = u8;
    const CAPACITY: usize = 255;

    fn is_full(&self) -> bool {
        self.len == 255
    }

    fn len(&self) -> usize {
        self.len as usize
    }

    fn total_size(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    fn new(value: u8) -> Self {
        let mut leaf = ByteLeaf {
            len: 1,
            bytes: [0; 255],
            next: std::ptr::null_mut(),
        };
        leaf.bytes[0] = value;
        leaf
    }

    fn split(&mut self) -> Box<Self> {
        self.len = 127;
        let mut leaf = Box::new(ByteLeaf {
            len: 128,
            bytes: [0; Self::CAPACITY],
            next: std::ptr::null_mut(),
        });
        // Why does the following not work??
        self.bytes[127..].swap_with_slice(&mut leaf.bytes[..128]);
        self.next = Box::into_raw(leaf);
        unsafe { Box::from_raw(self.next) }

    }

    fn insert(&mut self, index: usize, byte: u8) {
        unsafe {
            std::ptr::copy(
                &self.bytes[index],
                &mut self.bytes[index + 1],
                self.len as usize - index,
            );
        }
        self.len += 1;
        self.bytes[index] = byte;
    }
}

impl ByteLeaf {
    fn child_number(
        &self,
        index: usize,
        degree: usize,
        needle: u8,
    ) -> (usize, u8) {
        if index + degree > self.len as usize {
            if self.bytes[self.len as usize - 1] < needle {
                let next = unsafe { self.next.as_ref() }.unwrap();
                let rank = binary_search_rank(
                    needle,
                    index + degree - self.len as usize,
                    |i| next.bytes[i],
                );
                (self.len as usize - index + rank, next.bytes[rank])
            } else {
                let rank = binary_search_rank(
                    needle,
                    self.len as usize - index,
                    |i| self.bytes[index + i],
                );
                (rank, self.bytes[index + rank])
            }
        } else {
            let rank =
                binary_search_rank(needle, degree, |i| self.bytes[index + i]);
            (rank, self.bytes[index + rank])
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_bytetree_insert_begin() {
        let reference =
            (0..10000usize).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        let mut values = ByteTree::new();

        for (i, v) in reference.iter().rev().cloned().enumerate() {
            values.insert(0, v);
            assert_eq!(values.len(), i + 1);
        }

        for i in 0..10000 {
            assert_eq!(values.get(i), reference[i]);
        }
    }

    fn test_bytetree_insert_end() {
        let reference =
            (0..10000usize).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        let mut values = ByteTree::new();

        for (i, v) in reference.iter().rev().cloned().enumerate() {
            values.insert(values.len(), v);
            assert_eq!(values.len(), i + 1);
        }

        for i in 0..10000 {
            assert_eq!(values.get(i), reference[i]);
        }
    }
}
