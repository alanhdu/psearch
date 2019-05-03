use crate::tree::{Leaf, Tree};
use crate::utils::binary_search_rank_equal;

pub(super) type ByteTree = Tree<ByteLeaf>;

impl ByteTree {
    pub(crate) fn child_number(
        &self,
        index: usize,
        degree: usize,
        needle: u8,
    ) -> (usize, bool) {
        let (leaf, index) = self.get_leaf(index);
        leaf.child_number(index, degree, needle)
    }

    #[cfg(test)]
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
        self.bytes[127..].swap_with_slice(&mut leaf.bytes[..128]);
        self.next = Box::into_raw(leaf);
        unsafe { Box::from_raw(self.next) }
    }

    fn insert(&mut self, index: usize, byte: u8) {
        if index < 254 {
            unsafe {
                std::ptr::copy(
                    &self.bytes[index],
                    &mut self.bytes[index + 1],
                    self.len as usize - index,
                );
            }
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
    ) -> (usize, bool) {
        if index + degree > self.len as usize {
            if self.bytes[self.len as usize - 1] < needle {
                let next = unsafe { self.next.as_ref() }.unwrap();
                let (rank, found) = binary_search_rank_equal(
                    needle,
                    index + degree - self.len as usize,
                    |i| next.bytes[i],
                );
                (self.len as usize - index + rank, found)
            } else {
                binary_search_rank_equal(
                    needle,
                    self.len as usize - index,
                    |i| self.bytes[index + i],
                )
            }
        } else {
            binary_search_rank_equal(needle, degree, |i| self.bytes[index + i])
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_byteleaf_split() {
        let mut leaf = ByteLeaf {
            len: 255,
            bytes: [0xAA; 255],
            next: std::ptr::null_mut(),
        };
        let new = leaf.split();
        let raw = Box::into_raw(new);
        let new = unsafe { Box::from_raw(raw) };

        assert_eq!(leaf.next, raw);
        assert_eq!(leaf.len, 127);

        let mut expected = vec![0xAAu8; 127];
        expected.append(&mut vec![0; 128]);
        assert_eq!(&leaf.bytes as &[u8], &expected as &[u8]);

        assert_eq!(new.len, 128);
        assert!(new.next.is_null());
        let mut expected = vec![0xAAu8; 128];
        expected.append(&mut vec![0; 127]);
        assert_eq!(&new.bytes as &[u8], &expected as &[u8]);
    }

    #[test]
    fn test_byteleaf_insert_begin() {
        let mut leaf = ByteLeaf::new(255);
        let mut expected = vec![255u8];

        for i in 0..254 {
            leaf.insert(0, i);
            expected.insert(0, i);

            assert_eq!(leaf.len(), expected.len());
        }
        assert_eq!(&leaf.bytes as &[u8], &expected as &[u8]);
    }

    #[test]
    fn test_byteleaf_insert_end() {
        let mut leaf = ByteLeaf::new(255);
        let mut expected = vec![255u8];

        for i in 0..254 {
            leaf.insert(leaf.len(), i);
            expected.insert(expected.len(), i);

            assert_eq!(leaf.len(), expected.len());
        }
        assert_eq!(&leaf.bytes as &[u8], &expected as &[u8]);
    }

    #[test]
    fn test_byteleaf_insert_middle() {
        let mut leaf = ByteLeaf::new(255);
        let mut expected = vec![255u8];

        for i in 0..254 {
            leaf.insert(leaf.len() / 2, i);
            expected.insert(expected.len() / 2, i);

            assert_eq!(leaf.len(), expected.len());
        }
        assert_eq!(&leaf.bytes as &[u8], &expected as &[u8]);
    }

    #[test]
    fn test_bytetree_insert_begin() {
        let reference =
            (0..10000usize).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        let mut bytes = ByteTree::new();

        for (i, v) in reference.iter().rev().cloned().enumerate() {
            bytes.insert(0, v);
            assert_eq!(bytes.len(), i + 1);
        }

        for i in 0..10000 {
            assert_eq!(bytes.get(i), reference[i]);
        }
    }

    #[test]
    fn test_bytetree_insert_end() {
        let reference =
            (0..10000usize).map(|i| (i % 256) as u8).collect::<Vec<_>>();
        let mut bytes = ByteTree::new();

        for (i, v) in reference.iter().cloned().enumerate() {
            bytes.insert(bytes.len(), v);
            assert_eq!(bytes.len(), i + 1);
            assert_eq!(bytes.get(i), reference[i]);
        }

        for i in 0..10000 {
            assert_eq!(bytes.get(i), reference[i]);
        }
    }
}
