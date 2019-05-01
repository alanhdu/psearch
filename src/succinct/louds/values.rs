use crate::array::u32x16;
use crate::tree::{PackedPtr, Ptr, PtrMut};

const CAPACITY: usize = 16;
const LEAF_CAPACITY: usize = 64;

pub(super) struct ValueTree<T: Default> {
    root: Box<Node<T>>,
}

impl<T: Default> ValueTree<T> {
    pub(crate) fn new() -> ValueTree<T> {
        ValueTree {
            root: Box::new(Node {
                lens: [0; CAPACITY],
                ptrs: [PackedPtr::null(); CAPACITY],
            }),
        }
    }

    pub(crate) fn total_size(&self) -> usize {
        std::mem::size_of::<Self>() + self.root.total_size()
    }

    pub(crate) fn len(&self) -> usize {
        self.root.lens[CAPACITY - 1] as usize
    }

    pub(crate) fn get(&self, mut index: usize) -> &T {
        debug_assert!(index < self.len());
        let mut node: &Node<T> = &self.root;

        loop {
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
                    return &leaf[index];
                }
            }
        }
    }

    pub(crate) fn insert(&mut self, index: usize, value: T) {
        debug_assert!(index <= self.len());
        if index == 0 && self.len() == 0 {
            let mut leaf = Vec::with_capacity(LEAF_CAPACITY);
            leaf.push(value);
            self.root.ptrs[0] = PackedPtr::from_leaf(Box::new(leaf));
            self.root.lens = [1; CAPACITY];
            return;
        }

        let mut stack: Vec<(*mut Node<T>, usize)> = Vec::new();
        let mut node: &mut Node<T> = &mut self.root;

        let mut index = index as u32;
        loop {
            let rank = u32x16::rank(&node.lens, index) as usize;
            if rank > 0 {
                index -= node.lens[rank - 1];
            }
            u32x16::increment(&mut node.lens, rank);

            let n = &mut node.ptrs[rank] as *mut PackedPtr<Node<T>, Vec<T>>;
            match unsafe { &mut *n }.expand_mut() {
                PtrMut::None => unreachable!(),
                PtrMut::Inner(inner) => {
                    stack.push((node as *mut _, rank));
                    node = inner;
                }
                PtrMut::Leaf(leaf) => {
                    if leaf.len() >= LEAF_CAPACITY {
                        let mut new = leaf.split_off(LEAF_CAPACITY / 2);
                        if index as usize > LEAF_CAPACITY / 2 {
                            new.insert(
                                index as usize - LEAF_CAPACITY / 2,
                                value,
                            );
                        } else {
                            leaf.insert(index as usize, value);
                        }
                        stack.push((node, rank));
                        self.split(stack, new);
                    } else {
                        leaf.insert(index as usize, value);
                    }
                    break;
                }
            }
        }
    }

    fn split(&mut self, stack: Vec<(*mut Node<T>, usize)>, new: Vec<T>) {
        let mut ptr = PackedPtr::from_leaf(Box::new(new));

        for (node, rank) in stack.iter().rev().cloned() {
            let node = unsafe { &mut *node };
            if !node.is_full() {
                node.shift_right(rank);
                node.ptrs[rank + 1] = ptr;
                node.lens[rank] -= ptr.len() as u32;
                return;
            } else {
                let mut new = Box::new(node.split());

                if rank >= 8 {
                    new.shift_right(rank - 8);
                    new.ptrs[rank - 8 + 1] = ptr;
                    new.lens[rank - 8] -= ptr.len() as u32;
                } else {
                    node.shift_right(rank);
                    node.ptrs[rank + 1] = ptr;
                    node.lens[rank] -= ptr.len() as u32;
                }
                ptr = PackedPtr::from_inner(new);
            }
        }

        // We've recursed all the way to the root!
        debug_assert!(!self.root.is_full());
        debug_assert!(self.root.ptrs[9].is_null());

        let len = self.root.lens[CAPACITY - 1] + ptr.len() as u32;
        let root = std::mem::replace(
            &mut self.root,
            Box::new(Node {
                lens: [len as u32; CAPACITY],
                ptrs: [PackedPtr::null(); CAPACITY],
            }),
        );
        self.root.lens[0] -= ptr.len() as u32;
        self.root.ptrs[0] = PackedPtr::from_inner(root);
        self.root.ptrs[1] = ptr;
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Node<T> {
    lens: [u32; CAPACITY],
    ptrs: [PackedPtr<Node<T>, Vec<T>>; CAPACITY],
}

impl<T> Node<T> {
    fn total_size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        for ptr in self.ptrs.iter() {
            size += match ptr.expand() {
                Ptr::None => 0,
                Ptr::Leaf(leaf) => {
                    std::mem::size_of_val(leaf)
                        + leaf.capacity() * std::mem::size_of::<T>()
                }
                Ptr::Inner(_) => std::mem::size_of::<Node<T>>(),
            };
        }
        size
    }

    fn is_full(&self) -> bool {
        debug_assert_eq!(
            self.ptrs[CAPACITY - 1].is_null(),
            self.lens[CAPACITY - 1] == self.lens[CAPACITY - 2]
        );
        !self.ptrs[CAPACITY - 1].is_null()
    }

    fn split(&mut self) -> Node<T> {
        debug_assert!(self.is_full());
        let mut node = Node {
            lens: [0; CAPACITY],
            ptrs: [PackedPtr::null(); CAPACITY],
        };
        u32x16::split(&mut self.lens, &mut node.lens);
        self.ptrs[8..].swap_with_slice(&mut node.ptrs[..8]);
        node
    }

    fn shift_right(&mut self, rank: usize) {
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
        }
    }
}

impl<T> PackedPtr<Node<T>, Vec<T>> {
    fn len(self) -> usize {
        match self.expand() {
            Ptr::None => unreachable!(),
            Ptr::Leaf(l) => l.len(),
            Ptr::Inner(inner) => inner.lens[CAPACITY - 1] as usize,
        }
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
