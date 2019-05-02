use std::collections::{BTreeMap, VecDeque};
use std::iter::FromIterator;

use crate::select_rank::{SBitVec, SelectRank};

/// A Static LOUDS trie
pub struct SloudsTrie<T> {
    trie: SBitVec,
    has_value: SBitVec,
    bytes: Vec<u8>,
    values: Vec<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Cursor {
    bit_pos: usize,
    node_pos: usize,
}

impl Cursor {
    fn from_bit_pos(trie: &SBitVec, pos: usize) -> Cursor {
        Cursor {
            bit_pos: pos,
            node_pos: trie.rank0(pos),
        }
    }
}

impl<T> SloudsTrie<T> {
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<&T> {
        let mut cursor = Cursor {
            bit_pos: 0,
            node_pos: 0,
        };

        for byte in key.as_ref().iter() {
            if self.is_leaf(cursor.bit_pos) {
                return None;
            }

            let byte_begin = self.child(cursor.bit_pos, 0).node_pos - 1;
            let degree = self.degree(cursor.bit_pos);
            let slice = &self.bytes[byte_begin..byte_begin + degree];
            let child = slice.binary_search(byte).ok()?;
            cursor = self.child(cursor.bit_pos, child);
        }

        if self.has_value.get_bit(cursor.node_pos) {
            let value_pos = self.has_value.rank1(cursor.node_pos);
            self.values.get(value_pos)
        } else {
            None
        }
    }

    /// Get the bit-index of `cursor`'s `i`th child
    fn child(&self, cursor: usize, i: usize) -> Cursor {
        Cursor::from_bit_pos(
            &self.trie,
            self.trie.select0(self.trie.rank1(cursor + i)) + 1,
        )
    }

    fn is_leaf(&self, cursor: usize) -> bool {
        self.trie.get_bit(cursor) == false
    }

    fn degree(&self, cursor: usize) -> usize {
        if self.is_leaf(cursor) {
            0
        } else {
            let next = self.trie.select0(self.trie.rank0(cursor));
            next - cursor
        }
    }
}

/// A really bad trie implementation to construct the SloudsTrie trie
struct BadTrie<T> {
    children: BTreeMap<u8, BadTrie<T>>,
    value: Option<T>,
}

impl<T> BadTrie<T> {
    fn insert(&mut self, key: &[u8], value: T) {
        let mut node = self;
        for i in 0..(key.len() - 1) {
            let k = key[i];

            if !node.children.contains_key(&k) {
                node.children.insert(
                    k,
                    BadTrie {
                        children: BTreeMap::new(),
                        value: None,
                    },
                );
            }

            node = node.children.get_mut(&k).unwrap();
        }

        let k = key.last().unwrap();

        if !node.children.contains_key(k) {
            node.children.insert(
                *k,
                BadTrie {
                    children: BTreeMap::new(),
                    value: Some(value),
                },
            );
        } else {
            node.children.get_mut(k).unwrap().value = Some(value);
        }
    }
}

impl<T, K> FromIterator<(K, T)> for SloudsTrie<T>
where
    K: AsRef<[u8]>,
{
    fn from_iter<I>(input: I) -> Self
    where
        I: IntoIterator<Item = (K, T)>,
    {
        let mut trie = BadTrie {
            children: BTreeMap::new(),
            value: None,
        };
        for (key, value) in input.into_iter() {
            trie.insert(key.as_ref(), value);
        }

        let mut louds = Vec::new();
        let mut bytes: Vec<u8> = Vec::new();
        let mut values = Vec::new();
        let mut has_value = Vec::new();

        let mut queue = VecDeque::new();
        queue.push_back(&mut trie);
        while let Some(current) = queue.pop_front() {
            louds.append(&mut vec![true; current.children.len()]);
            louds.push(false);

            bytes.extend(current.children.keys());
            if let Some(value) = current.value.take() {
                values.push(value);
                has_value.push(true);
            } else {
                has_value.push(false);
            }

            queue.extend(current.children.values_mut());
        }

        SloudsTrie {
            trie: SBitVec::from_iter(louds),
            has_value: SBitVec::from_iter(has_value),
            bytes,
            values,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_slouds_from_iter() {
        let keys: [&[u8]; 11] = [
            b"bel", b"bem", b"ben", b"bf", b"cg", b"dho", b"di", b"djp",
            b"djq", b"dk", b"b",
        ];

        // We are encoding the following tree:
        //                 *
        //              /  |    \
        // 1         b     c      d
        //          / \    |   / / \ \
        // 4       e   f   g  h i   j k
        //        /|\         |    / \
        // 11    l m n        o   p   q
        let slouds = SloudsTrie::from_iter(keys.iter().map(|k| (k, ())));

        assert_eq!(
            slouds.trie.to_vec(),
            vec![
                true, true, true, false, // root
                true, true, false, // b
                true, false, // c
                true, true, true, true, false, // d
                true, true, true, false, // e
                false, false, // f and g
                true, false, // h
                false, // i
                true, true, false, // j
                false, false, false, false, //k, l, m, n,
                false, false, false, // o, p, q
            ]
        );
        // Remember to include the root!
        assert_eq!(
            slouds.has_value.to_vec(),
            vec![
                false, true, false, false, false, true, true, false, true,
                false, true, true, true, true, true, true, true
            ]
        );
        assert_eq!(
            slouds.bytes,
            vec![
                b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k',
                b'l', b'm', b'n', b'o', b'p', b'q'
            ]
        );
        assert_eq!(slouds.values, vec![(); 11]);
    }

    #[test]
    #[rustfmt::skip]
    fn test_slouds_traverse() {
        let root = Cursor {
            bit_pos: 0,
            node_pos: 0,
        };
        // We are encoding the following tree:
        //                 *
        //              /  |    \
        // 1         b     c      d
        //          / \    |   / / \ \
        // 4       e   f   g  h i   j k
        //        /|\         |    / \
        // 11    l m n        o   p   q
        let keys: [&[u8]; 11] = [
            b"bel", b"bem", b"ben", b"bf", b"cg", b"dho", b"di", b"djp",
            b"djq", b"dk", b"b",
        ];
        let slouds = SloudsTrie::from_iter(keys.iter().map(|k| (k, k[0])));

        assert_eq!(slouds.degree(root.bit_pos), 3);

        let b = slouds.child(root.bit_pos, 0);
        assert_eq!(b, Cursor { bit_pos: 4, node_pos: 1});
        assert_eq!(slouds.degree(b.bit_pos), 2);

        let c = slouds.child(root.bit_pos, 1);
        assert_eq!(c, Cursor { bit_pos: 7, node_pos: 2});
        assert_eq!(slouds.degree(c.bit_pos), 1);

        let d = slouds.child(root.bit_pos, 2);
        assert_eq!(d, Cursor { bit_pos: 9, node_pos: 3});
        assert_eq!(slouds.degree(d.bit_pos), 4);

        let e = slouds.child(b.bit_pos, 0);
        assert_eq!(e, Cursor { bit_pos: 14, node_pos: 4 });
        assert_eq!(slouds.degree(e.bit_pos), 3);

        let f = slouds.child(b.bit_pos, 1);
        assert_eq!(f, Cursor { bit_pos: 18, node_pos: 5 });
        assert_eq!(slouds.degree(f.bit_pos), 0);

        let g = slouds.child(c.bit_pos, 0);
        assert_eq!(g, Cursor { bit_pos: 19, node_pos: 6 });
        assert_eq!(slouds.degree(g.bit_pos), 0);

        let h = slouds.child(d.bit_pos, 0);
        assert_eq!(h, Cursor { bit_pos: 20, node_pos: 7 });
        assert_eq!(slouds.degree(h.bit_pos), 1);

        let i = slouds.child(d.bit_pos, 1);
        assert_eq!(i, Cursor { bit_pos: 22, node_pos: 8 });
        assert_eq!(slouds.degree(i.bit_pos), 0);

        let j = slouds.child(d.bit_pos, 2);
        assert_eq!(j, Cursor { bit_pos: 23, node_pos: 9 });
        assert_eq!(slouds.degree(j.bit_pos), 2);

        let k = slouds.child(d.bit_pos, 3);
        assert_eq!(k, Cursor { bit_pos: 26, node_pos: 10 });
        assert_eq!(slouds.degree(k.bit_pos), 0);

        let l = slouds.child(e.bit_pos, 0);
        assert_eq!(l, Cursor { bit_pos: 27, node_pos: 11 });
        assert_eq!(slouds.degree(l.bit_pos), 0);

        let m = slouds.child(e.bit_pos, 1);
        assert_eq!(m, Cursor { bit_pos: 28, node_pos: 12 });
        assert_eq!(slouds.degree(m.bit_pos), 0);

        let n = slouds.child(e.bit_pos, 2);
        assert_eq!(n, Cursor { bit_pos: 29, node_pos: 13 });
        assert_eq!(slouds.degree(n.bit_pos), 0);

        let o = slouds.child(h.bit_pos, 0);
        assert_eq!(o, Cursor { bit_pos: 30, node_pos: 14 });
        assert_eq!(slouds.degree(o.bit_pos), 0);

        let p = slouds.child(j.bit_pos, 0);
        assert_eq!(p, Cursor { bit_pos: 31, node_pos: 15 });
        assert_eq!(slouds.degree(p.bit_pos), 0);

        let q = slouds.child(j.bit_pos, 1);
        assert_eq!(q, Cursor { bit_pos: 32, node_pos: 16 });
        assert_eq!(slouds.degree(q.bit_pos), 0);
    }

    #[test]
    fn test_slouds_get() {
        let keys: [&[u8]; 11] = [
            b"bel", b"bem", b"ben", b"bf", b"cg", b"dho", b"di", b"djp",
            b"djq", b"dk", b"b",
        ];

        let slouds = SloudsTrie::from_iter(keys.iter().map(|k| (k, k[0])));

        for key in keys.iter() {
            assert_eq!(slouds.get(key), Some(&key[0]));
        }

        assert_eq!(slouds.get(b"belarus"), None);
        assert_eq!(slouds.get(b"dh"), None);
        assert_eq!(slouds.get(b"dj"), None);
    }

    #[test]
    fn test_slouds_get_numbers() {
        let numbers: [u16; 25] = [
            9424, 12398, 54780, 51835, 63026, 8401, 63521, 49588, 14290, 60102,
            12443, 35584, 11924, 55247, 770, 20443, 1862, 11155, 25753, 7685,
            1900, 7743, 43659, 63103, 3614,
        ];

        let slouds =
            SloudsTrie::from_iter(numbers.iter().map(|k| (k.to_be_bytes(), k)));

        for k in numbers.iter() {
            assert_eq!(slouds.get(k.to_be_bytes()), Some(&k));
        }
    }
}
