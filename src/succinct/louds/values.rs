use crate::tree::{Leaf, Tree};

const CAPACITY: usize = 16;

pub(super) type ValueTree<T> = Tree<Vec<T>>;

impl<T> ValueTree<T> {
    pub(crate) fn get(&self, index: usize) -> &T {
        let (leaf, index) = self.get_leaf(index);
        &leaf[index]
    }

    pub(crate) fn set(&mut self, index: usize, value: T) -> T {
        let (leaf, index) = self.get_leaf_mut(index);
        std::mem::replace(&mut leaf[index], value)
    }

    #[cfg(test)]
    pub(crate) fn to_vec(&self) -> Vec<&T> {
        (0..self.len()).map(|i| self.get(i)).collect::<Vec<_>>()
    }
}

impl<T> Leaf for Vec<T> {
    type Output = T;
    const CAPACITY: usize = 64;

    fn total_size(&self) -> usize {
        std::mem::size_of::<T>() * self.capacity() + std::mem::size_of::<Self>()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn new(value: T) -> Self {
        let mut vec = Vec::with_capacity(Self::CAPACITY);
        vec.push(value);
        vec
    }

    fn split(&mut self) -> Box<Self> {
        Box::new(self.split_off(Self::CAPACITY / 2))
    }

    fn insert(&mut self, index: usize, value: T) {
        self.insert(index, value);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_insert_begin() {
        let reference = (0..10000usize).collect::<Vec<_>>();
        let mut values = ValueTree::new();

        for (i, v) in reference.iter().rev().cloned().enumerate() {
            values.insert(0, v);
            assert_eq!(values.len(), i + 1);
        }

        for i in 0..10000 {
            assert_eq!(*values.get(i), reference[i]);
        }
    }

    #[test]
    fn test_insert_end() {
        let reference = (0..10000).collect::<Vec<_>>();
        let mut values = ValueTree::new();

        for (i, v) in reference.iter().cloned().enumerate() {
            values.insert(i, v);
            assert_eq!(values.len(), i + 1);
        }

        for i in 0..10000 {
            assert_eq!(*values.get(i), reference[i]);
        }
    }
}
