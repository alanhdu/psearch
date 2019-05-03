mod ptr;

use crate::array::u32x16;
pub(crate) use ptr::{PackedPtr, Ptr, PtrMut};
use std::ptr::NonNull;

const CAPACITY: usize = 16;

pub(crate) struct Tree<L: Leaf> {
    root: Box<Node<L>>,
}

impl<L: Leaf> Tree<L> {
    pub(crate) fn new() -> Tree<L> {
        Tree {
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

    pub(crate) fn get_leaf(&self, mut index: usize) -> (&L, usize) {
        debug_assert!(index < self.len());
        let mut node: &Node<L> = &self.root;

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
                    return (leaf, index);
                }
            }
        }
    }

    pub(crate) fn get_leaf_mut(&mut self, mut index: usize) -> (&mut L, usize) {
        debug_assert!(index < self.len());
        let mut node: &mut Node<L> = &mut self.root;

        loop {
            let rank = u32x16::rank(&node.lens, 1 + index as u32) as usize;
            if rank > 0 {
                index -= node.lens[rank - 1] as usize;
            }

            match node.ptrs[rank].expand_mut() {
                PtrMut::None => unreachable!(),
                PtrMut::Inner(inner) => {
                    node = inner;
                }
                PtrMut::Leaf(leaf) => {
                    return (leaf, index);
                }
            }
        }
    }

    pub(crate) fn insert(&mut self, index: usize, value: L::Output) {
        debug_assert!(index <= self.len());
        if index == 0 && self.len() == 0 {
            self.root.ptrs[0] = PackedPtr::from_leaf(Box::new(L::new(value)));
            self.root.lens = [1; CAPACITY];
            return;
        }

        let mut stack: Vec<(NonNull<Node<L>>, usize)> = Vec::new();
        let mut node: &mut Node<L> = &mut self.root;
        let mut index = index as u32;
        loop {
            let rank = u32x16::rank(&node.lens, index) as usize;
            if rank > 0 {
                index -= node.lens[rank - 1];
            }
            u32x16::increment(&mut node.lens, rank);

            let n = &mut node.ptrs[rank] as *mut PackedPtr<Node<L>, L>;
            match unsafe { &mut *n }.expand_mut() {
                PtrMut::None => unreachable!(),
                PtrMut::Inner(inner) => {
                    stack.push((NonNull::from(node), rank));
                    node = inner;
                }
                PtrMut::Leaf(leaf) => {
                    if leaf.is_full() {
                        let mut new = leaf.split();
                        if index as usize > L::CAPACITY / 2 {
                            new.insert(index as usize - L::CAPACITY / 2, value);
                        } else {
                            leaf.insert(index as usize, value);
                        }
                        stack.push((NonNull::from(node), rank));
                        self.split(stack, new);
                    } else {
                        leaf.insert(index as usize, value);
                    }
                    break;
                }
            }
        }
    }

    fn split(&mut self, stack: Vec<(NonNull<Node<L>>, usize)>, new: Box<L>) {
        let mut ptr = PackedPtr::from_leaf(new);

        for (node, rank) in stack.iter().rev().cloned() {
            let node = unsafe { &mut *node.as_ptr() };
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

struct Node<L: Leaf> {
    lens: [u32; CAPACITY],
    ptrs: [PackedPtr<Node<L>, L>; CAPACITY],
}

impl<L: Leaf> Drop for Node<L> {
    fn drop(&mut self) {
        // TODO: avoid recursive drop
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

impl<L: Leaf> Node<L> {
    fn total_size(&self) -> usize {
        let mut size = std::mem::size_of::<Self>();
        for ptr in self.ptrs.iter() {
            size += match ptr.expand() {
                Ptr::None => 0,
                Ptr::Leaf(leaf) => leaf.total_size(),
                Ptr::Inner(_) => std::mem::size_of::<Node<L>>(),
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

    fn split(&mut self) -> Node<L> {
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

impl<L: Leaf> PackedPtr<Node<L>, L> {
    pub(crate) fn len(self) -> usize {
        match self.expand() {
            Ptr::None => unreachable!(),
            Ptr::Leaf(l) => l.len(),
            Ptr::Inner(inner) => inner.lens[CAPACITY - 1] as usize,
        }
    }
}

pub(crate) trait Leaf {
    type Output;
    const CAPACITY: usize;

    fn total_size(&self) -> usize;

    fn len(&self) -> usize;
    fn is_full(&self) -> bool {
        self.len() == Self::CAPACITY
    }

    fn new(value: Self::Output) -> Self;
    fn split(&mut self) -> Box<Self>;
    fn insert(&mut self, index: usize, value: Self::Output);
}
