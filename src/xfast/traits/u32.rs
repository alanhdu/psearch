use std::collections::{hash_map::Entry, HashMap};
use std::ptr;

use super::{Descendant, LNode, LevelSearchable};

impl<T> LevelSearchable<T> for u32 {
    type LSS = LevelSearch<T>;
    const MIN: u32 = 0;
    const MAX: u32 = 0;

    fn lss_new() -> LevelSearch<T> {
        LevelSearch::new()
    }

    fn lss_clear(lss: &mut LevelSearch<T>) {
        lss.clear();
    }

    fn lss_insert(lss: &mut LevelSearch<T>, node: &mut LNode<u32, T>) {
        lss.insert(node);
    }

    fn lss_remove(lss: &mut LevelSearch<T>, node: &LNode<u32, T>) {
        lss.remove(node);
    }

    fn lss_longest_descendant(
        lss: &LevelSearch<T>,
        key: Self,
    ) -> (u8, &Descendant<u32, T>) {
        lss.longest_descendant(key)
    }
}

#[derive(Debug, Default, Eq, PartialEq)]
pub struct LevelSearch<T> {
    l0: Descendant<u32, T>,
    l1: HashMap<[u8; 1], Descendant<u32, T>>,
    l2: HashMap<[u8; 2], Descendant<u32, T>>,
    l3: HashMap<[u8; 3], Descendant<u32, T>>,
}

impl<T> LevelSearch<T> {
    fn new() -> LevelSearch<T> {
        LevelSearch {
            l0: Descendant::new(),
            l1: HashMap::default(),
            l2: HashMap::default(),
            l3: HashMap::default(),
        }
    }

    fn clear(&mut self) {
        self.l0 = Descendant::new();
        self.l1.clear();
        self.l2.clear();
        self.l3.clear();
    }

    fn insert(&mut self, node: &mut LNode<u32, T>) {
        let bytes = node.key.to_be_bytes();
        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];

        // Do not use longest_descendant so we can re-use the hashes and
        // entries
        let mut v1 = self.l1.entry(b1);
        let mut v2 = self.l2.entry(b2);
        let mut v3 = self.l3.entry(b3);

        if let Entry::Occupied(ref mut o) = v2 {
            if let Entry::Occupied(ref mut o) = v3 {
                o.get_mut().set_links(bytes[3], node);
            } else {
                o.get_mut().set_links(bytes[2], node);
            }
        } else if let Entry::Occupied(ref mut o) = v1 {
            o.get_mut().set_links(bytes[1], node);
        } else {
            self.l0.set_links(bytes[0], node);
        }

        fn insert_into_entry<T, K>(
            byte: u8,
            node: &mut LNode<u32, T>,
            entry: Entry<K, Descendant<u32, T>>,
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

    fn remove(&mut self, node: &LNode<u32, T>) {
        let bytes = node.key.to_be_bytes();
        let b1 = [bytes[0]];
        let b2 = [bytes[0], bytes[1]];
        let b3 = [bytes[0], bytes[1], bytes[2]];

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
    }

    fn longest_descendant(&self, key: u32) -> (u8, &Descendant<u32, T>) {
        let bytes = key.to_be_bytes();
        if let Some(desc) = self.l2.get(&[bytes[0], bytes[1]]) {
            if let Some(desc) = self.l3.get(&[bytes[0], bytes[1], bytes[2]]) {
                (bytes[3], &desc)
            } else {
                (bytes[2], &desc)
            }
        } else if let Some(desc) = self.l1.get(&[bytes[0]]) {
            (bytes[1], &desc)
        } else {
            (bytes[0], &self.l0)
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::BTreeMap;
    use std::iter::FromIterator;

    #[test]
    fn test_levelsearch_insert_1() {
        let mut lss = LevelSearch::new();
        let mut node = LNode::new(0xdeadbeef, ());

        lss.insert(&mut node);

        let ptr = &mut node as *mut _;
        let nonnull = ptr::NonNull::new(ptr).unwrap();

        assert_eq!(
            lss.l0.bounds,
            BTreeMap::from_iter(vec![(0xde, (nonnull, nonnull)),])
        );

        assert_eq!(
            lss.l1,
            HashMap::from_iter(vec![(
                [0xde],
                Descendant {
                    bounds: BTreeMap::from_iter(vec![(
                        0xad,
                        (nonnull, nonnull)
                    ),])
                }
            )])
        );

        assert_eq!(
            lss.l2,
            HashMap::from_iter(vec![(
                [0xde, 0xad],
                Descendant {
                    bounds: BTreeMap::from_iter(vec![(
                        0xbe,
                        (nonnull, nonnull)
                    ),])
                }
            )])
        );

        assert_eq!(
            lss.l3,
            HashMap::from_iter(vec![(
                [0xde, 0xad, 0xbe],
                Descendant {
                    bounds: BTreeMap::from_iter(vec![(
                        0xef,
                        (nonnull, nonnull)
                    ),])
                }
            )])
        );
    }

    #[test]
    fn test_levelsearch_insert_4() {
        let mut lss = LevelSearch::new();
        let mut n1 = LNode::new(0xbaadf00d, ());
        let mut n2 = LNode::new(0xdeadbeef, ());
        let mut n3 = LNode::new(0xdeadc0de, ());
        let mut n4 = LNode::new(0xdeadc0fe, ());

        lss.insert(&mut n3);
        lss.insert(&mut n4);
        lss.insert(&mut n2);
        lss.insert(&mut n1);

        let p1 = &mut n1 as *mut _;
        let p2 = &mut n2 as *mut _;
        let p3 = &mut n3 as *mut _;
        let p4 = &mut n4 as *mut _;

        assert!(n1.prev.is_null());
        assert_eq!(n1.next, p2);
        assert_eq!(n2.prev, p1);
        assert_eq!(n2.next, p3);
        assert_eq!(n3.prev, p2);
        assert_eq!(n3.next, p4);
        assert_eq!(n4.prev, p3);
        assert!(n4.next.is_null());

        let nn1 = ptr::NonNull::new(p1).unwrap();
        let nn2 = ptr::NonNull::new(p2).unwrap();
        let nn3 = ptr::NonNull::new(p3).unwrap();
        let nn4 = ptr::NonNull::new(p4).unwrap();

        assert_eq!(
            lss.l0.bounds,
            BTreeMap::from_iter(vec![(0xba, (nn1, nn1)), (0xde, (nn2, nn4))])
        );
        assert_eq!(
            lss.l1,
            HashMap::from_iter(vec![
                (
                    [0xba],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xad, (nn1, nn1))])
                    }
                ),
                (
                    [0xde],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xad, (nn2, nn4))])
                    }
                )
            ])
        );

        assert_eq!(
            lss.l2,
            HashMap::from_iter(vec![
                (
                    [0xba, 0xad],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xf0, (nn1, nn1))])
                    }
                ),
                (
                    [0xde, 0xad],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![
                            (0xbe, (nn2, nn2)),
                            (0xc0, (nn3, nn4)),
                        ])
                    }
                )
            ])
        );
        assert_eq!(
            lss.l3,
            HashMap::from_iter(vec![
                (
                    [0xba, 0xad, 0xf0],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0x0d, (nn1, nn1))])
                    }
                ),
                (
                    [0xde, 0xad, 0xbe],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![(0xef, (nn2, nn2))])
                    }
                ),
                (
                    [0xde, 0xad, 0xc0],
                    Descendant {
                        bounds: BTreeMap::from_iter(vec![
                            (0xde, (nn3, nn3)),
                            (0xfe, (nn4, nn4)),
                        ])
                    }
                ),
            ])
        );
    }

}
