#![allow(dead_code)]
mod u32;

use std::collections::btree_map::Entry as BTreeEntry;
use std::collections::BTreeMap;
use std::ptr;

pub trait LevelSearchable<T>:
    std::hash::Hash + Copy + Clone + Eq + PartialEq + Ord + PartialOrd
{
    type LSS;

    const MIN: Self;
    const MAX: Self;

    // No GATs, so can't implement LSS with a trait bound. Instead, add
    // LSS methods to this trait.
    fn lss_new() -> Self::LSS;
    fn lss_clear(lss: &mut Self::LSS);
    fn lss_insert(lss: &mut Self::LSS, node: &mut LNode<Self, T>);
    fn lss_remove(lss: &mut Self::LSS, node: &LNode<Self, T>);
    fn lss_longest_descendant(
        lss: &Self::LSS,
        key: Self,
    ) -> (u8, &Descendant<Self, T>);

    fn lss_min(lss: &Self::LSS) -> Option<&LNode<Self, T>> {
        let (_, desc) = Self::lss_longest_descendant(lss, Self::MIN);
        desc.successor(0)
    }

    fn lss_predecessor(lss: &Self::LSS, key: Self) -> Option<&LNode<Self, T>> {
        let (byte, desc) = Self::lss_longest_descendant(lss, key);
        desc.predecessor(byte).or_else(|| {
            desc.successor(byte)
                .and_then(|node| unsafe { node.prev.as_ref() })
        })
    }

    fn lss_successor(lss: &Self::LSS, key: Self) -> Option<&LNode<Self, T>> {
        let (byte, desc) = Self::lss_longest_descendant(lss, key);
        desc.successor(byte).or_else(|| {
            desc.predecessor(byte)
                .and_then(|node| unsafe { node.next.as_ref() })
        })
    }
}

type Ptr<K, V> = ptr::NonNull<LNode<K, V>>;

#[derive(Debug, Default, Eq, PartialEq)]
pub struct Descendant<K: LevelSearchable<V>, V> {
    bounds: BTreeMap<u8, (Ptr<K, V>, Ptr<K, V>)>,
}

impl<K: LevelSearchable<V>, V> Descendant<K, V> {
    fn new() -> Descendant<K, V> {
        Descendant {
            bounds: BTreeMap::new(),
        }
    }

    fn is_empty(&self) -> bool {
        self.bounds.is_empty()
    }

    /// Find the predecessor of byte, assuming byte has at most 1 child
    fn predecessor(&self, byte: u8) -> Option<&LNode<K, V>> {
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
    fn predecessor_mut(&mut self, byte: u8) -> Option<&mut LNode<K, V>> {
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
    fn successor_mut(&mut self, byte: u8) -> Option<&mut LNode<K, V>> {
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
    fn successor(&self, byte: u8) -> Option<&LNode<K, V>> {
        self.bounds.range(byte..).next().map(|(&b, (min, max))| {
            if b == byte {
                debug_assert_eq!(min, max);
            }
            unsafe { min.as_ref() }
        })
    }

    /// If this is the "lowest" Descendant matching the prefix, insert
    /// node into the linked list.
    fn set_links(&mut self, byte: u8, node: &mut LNode<K, V>) {
        if let Some(next) = self.successor_mut(byte) {
            debug_assert!(next.key > node.key);
            node.set_next(next);
        } else if let Some(prev) = self.predecessor_mut(byte) {
            debug_assert!(prev.key < node.key);
            node.set_prev(prev);
        }
    }

    /// Insert (byte, node), return whether it is a border node
    fn merge(&mut self, byte: u8, node: &mut LNode<K, V>) -> bool {
        match self.bounds.entry(byte) {
            BTreeEntry::Vacant(v) => {
                v.insert(unsafe {
                    (
                        ptr::NonNull::new_unchecked(node),
                        ptr::NonNull::new_unchecked(node),
                    )
                });
                true
            }
            BTreeEntry::Occupied(mut o) => {
                let (min, max) = o.get_mut();
                if node.key < unsafe { min.as_ref() }.key {
                    *min = ptr::NonNull::from(node);
                    true
                } else if node.key > unsafe { max.as_ref() }.key {
                    *max = ptr::NonNull::from(node);
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Remove the byte/node pair from the descendant pointers
    fn remove(&mut self, byte: u8, node: &LNode<K, V>) {
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
pub struct LNode<K: LevelSearchable<V>, V> {
    pub(super) key: K,
    pub(super) value: V,

    pub(super) prev: *mut LNode<K, V>,
    pub(super) next: *mut LNode<K, V>,
}

impl<V, K: LevelSearchable<V>> LNode<K, V> {
    pub(super) fn new(key: K, value: V) -> LNode<K, V> {
        LNode {
            key,
            value,
            prev: ptr::null_mut(),
            next: ptr::null_mut(),
        }
    }

    fn set_prev(&mut self, other: &mut LNode<K, V>) {
        self.prev = other;
        self.next = other.next;

        other.next = self;
        if let Some(next) = unsafe { self.next.as_mut() } {
            next.prev = self;
        }
    }

    fn set_next(&mut self, other: &mut LNode<K, V>) {
        self.next = other;
        self.prev = other.prev;

        other.prev = self;
        if let Some(prev) = unsafe { self.prev.as_mut() } {
            prev.next = self;
        }
    }
}
