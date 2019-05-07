use std::collections::{hash_map::Entry, HashMap};
use std::ptr;

use super::{Descendant, LNode, LevelSearchable};

impl<T> LevelSearchable<T> for u64 {
    type LSS = LevelSearch<T>;
    const MIN: u64 = 0;
    const MAX: u64 = 0;

    fn lss_new() -> LevelSearch<T> {
        LevelSearch::new()
    }

    fn lss_clear(lss: &mut LevelSearch<T>) {
        lss.clear();
    }

    fn lss_insert(lss: &mut LevelSearch<T>, node: &mut LNode<u64, T>) {
        lss.insert(node);
    }

    fn lss_remove(lss: &mut LevelSearch<T>, node: &LNode<u64, T>) {
        lss.remove(node);
    }

    fn lss_longest_descendant(
        lss: &LevelSearch<T>,
        key: Self,
    ) -> (u8, &Descendant<u64, T>) {
        lss.longest_descendant(key)
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct LevelSearch<T> {
    l0: Descendant<u64, T>,
    l1: HashMap<[u8; 1], Descendant<u64, T>>,
    l2: HashMap<[u8; 2], Descendant<u64, T>>,
    l3: HashMap<[u8; 3], Descendant<u64, T>>,
    l4: HashMap<[u8; 4], Descendant<u64, T>>,
    l5: HashMap<[u8; 5], Descendant<u64, T>>,
    l6: HashMap<[u8; 6], Descendant<u64, T>>,
    l7: HashMap<[u8; 7], Descendant<u64, T>>,
}

impl<T> LevelSearch<T> {
    fn new() -> LevelSearch<T> {
        LevelSearch {
            l0: Descendant::new(),
            l1: HashMap::default(),
            l2: HashMap::default(),
            l3: HashMap::default(),
            l4: HashMap::default(),
            l5: HashMap::default(),
            l6: HashMap::default(),
            l7: HashMap::default(),
        }
    }

    fn clear(&mut self) {
        self.l0 = Descendant::new();
        self.l1.clear();
        self.l2.clear();
        self.l3.clear();
    }

    fn insert(&mut self, node: &mut LNode<u64, T>) {
        let bytes = node.key.to_be_bytes();
        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];
        let b4 = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let b5 = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]];
        let b6 = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
        let b7 = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
            bytes[6],
        ];

        // Do not use longest_descendant so we can re-use the hashes and
        // entries
        let mut v1 = self.l1.entry(b1);
        let mut v2 = self.l2.entry(b2);
        let mut v3 = self.l3.entry(b3);
        let mut v4 = self.l4.entry(b4);
        let mut v5 = self.l5.entry(b5);
        let mut v6 = self.l6.entry(b6);
        let mut v7 = self.l7.entry(b7);

        if let Entry::Occupied(ref mut o4) = v4 {
            if let Entry::Occupied(ref mut o6) = v6 {
                if let Entry::Occupied(ref mut o7) = v7 {
                    o7.get_mut().set_links(bytes[7], node);
                } else {
                    o6.get_mut().set_links(bytes[6], node);
                }
            } else {
                if let Entry::Occupied(ref mut o5) = v5 {
                    o5.get_mut().set_links(bytes[5], node);
                } else {
                    o4.get_mut().set_links(bytes[4], node);
                }
            }
        } else {
            if let Entry::Occupied(ref mut o2) = v2 {
                if let Entry::Occupied(ref mut o3) = v3 {
                    o3.get_mut().set_links(bytes[3], node);
                } else {
                    o2.get_mut().set_links(bytes[2], node);
                }
            } else if let Entry::Occupied(ref mut o1) = v1 {
                o1.get_mut().set_links(bytes[1], node);
            } else {
                self.l0.set_links(bytes[0], node);
            }
        }

        fn insert_into_entry<T, K>(
            byte: u8,
            node: &mut LNode<u64, T>,
            entry: Entry<K, Descendant<u64, T>>,
        ) -> bool {
            match entry {
                Entry::Vacant(v) => {
                    let mut desc = Descendant::new();
                    desc.bounds.insert(byte, unsafe {
                        (
                            ptr::NonNull::new_unchecked(node),
                            ptr::NonNull::new_unchecked(node),
                        )
                    });
                    v.insert(desc);
                    true
                }
                Entry::Occupied(mut o) => o.get_mut().merge(byte, node),
            }
        }
        if !insert_into_entry(bytes[7], node, v7) {
            return;
        }
        if !insert_into_entry(bytes[6], node, v6) {
            return;
        }
        if !insert_into_entry(bytes[5], node, v5) {
            return;
        }
        if !insert_into_entry(bytes[4], node, v4) {
            return;
        }
        if !insert_into_entry(bytes[3], node, v3) {
            return;
        }
        if !insert_into_entry(bytes[2], node, v2) {
            return;
        }
        if !insert_into_entry(bytes[1], node, v1) {
            return;
        }
        self.l0.merge(bytes[0], node);
    }

    fn remove(&mut self, node: &LNode<u64, T>) {
        let bytes = node.key.to_be_bytes();
        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];
        let b4 = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let b5 = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]];
        let b6 = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
        let b7 = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
            bytes[6],
        ];

        self.l0.remove(bytes[0], node);
        if let Entry::Occupied(mut o) = self.l1.entry(b1) {
            o.get_mut().remove(bytes[1], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let Entry::Occupied(mut o) = self.l2.entry(b2) {
            o.get_mut().remove(bytes[2], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let Entry::Occupied(mut o) = self.l3.entry(b3) {
            o.get_mut().remove(bytes[3], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let Entry::Occupied(mut o) = self.l4.entry(b4) {
            o.get_mut().remove(bytes[4], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let Entry::Occupied(mut o) = self.l5.entry(b5) {
            o.get_mut().remove(bytes[5], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let Entry::Occupied(mut o) = self.l6.entry(b6) {
            o.get_mut().remove(bytes[6], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
        if let Entry::Occupied(mut o) = self.l7.entry(b7) {
            o.get_mut().remove(bytes[7], node);
            if o.get().is_empty() {
                o.remove();
            }
        }
    }

    fn longest_descendant(&self, key: u64) -> (u8, &Descendant<u64, T>) {
        let bytes = key.to_be_bytes();

        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];
        let b4 = [bytes[0], bytes[1], bytes[2], bytes[3]];
        let b5 = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4]];
        let b6 = [bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5]];
        let b7 = [
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5],
            bytes[6],
        ];

        if let Some(desc) = self.l4.get(&b4) {
            if let Some(desc) = self.l6.get(&b6) {
                if let Some(desc) = self.l7.get(&b7) {
                    (bytes[7], &desc)
                } else {
                    (bytes[6], &desc)
                }
            } else if let Some(desc) = self.l5.get(&b5) {
                (bytes[5], &desc)
            } else {
                (bytes[4], &desc)
            }
        } else {
            if let Some(desc) = self.l2.get(&b2) {
                if let Some(desc) = self.l3.get(&b3) {
                    (bytes[3], &desc)
                } else {
                    (bytes[2], &desc)
                }
            } else if let Some(desc) = self.l1.get(&b1) {
                (bytes[1], &desc)
            } else {
                (bytes[0], &self.l0)
            }
        }
    }
}
