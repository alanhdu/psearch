use crate::select_rank::{BitVec, SelectRank};
use std::iter::FromIterator;

mod bytes;
mod values;

pub struct LoudsTrie<T> {
    trie: BitVec,
    has_value: BitVec,
    bytes: bytes::ByteTree,
    values: values::ValueTree<T>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Cursor {
    bit_pos: usize,
    node_pos: usize,
}

impl Cursor {
    fn from_bit_pos(trie: &BitVec, pos: usize) -> Cursor {
        Cursor {
            bit_pos: pos,
            node_pos: trie.rank0(pos),
        }
    }
}

impl<T> LoudsTrie<T> {
    pub fn new() -> LoudsTrie<T> {
        let mut louds = LoudsTrie {
            trie: BitVec::new(),
            has_value: BitVec::new(),
            bytes: bytes::ByteTree::new(),
            values: values::ValueTree::new(),
        };
        louds.trie.insert(0, false);
        louds.has_value.insert(0, false);
        louds
    }

    pub fn total_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + self.bytes.total_size()
            + self.trie.total_size()
            + self.has_value.total_size()
            + self.values.total_size()
    }

    pub fn insert<K: AsRef<[u8]>>(&mut self, key: K, value: T) -> Option<T> {
        let mut cursor = Cursor {
            bit_pos: 0,
            node_pos: 0,
        };

        for byte in key.as_ref().iter().cloned() {
            if self.is_leaf(cursor.bit_pos) {
                let child = self.trie.select0(self.trie.rank1(cursor.bit_pos));
                self.trie.insert(child, false);
                self.trie.insert(cursor.bit_pos, true);
                let byte_begin = self.child(cursor.bit_pos, 0).node_pos - 1;
                self.bytes.insert(byte_begin, byte);

                cursor = self.child(cursor.bit_pos, 0);
                self.has_value.insert(cursor.node_pos, false);
            } else {
                let byte_begin = self.child(cursor.bit_pos, 0).node_pos - 1;
                let degree = self.degree(cursor.bit_pos);
                let (child_number, found) =
                    self.bytes.child_number(byte_begin, degree, byte);
                if !found {
                    let child = self.trie.select0(
                        self.trie.rank1(cursor.bit_pos + child_number),
                    );
                    debug_assert!(child > cursor.bit_pos);

                    self.trie.insert(child, false);
                    self.trie.insert(cursor.bit_pos, true);
                    self.bytes.insert(byte_begin + child_number, byte);

                    cursor = self.child(cursor.bit_pos, child_number);
                    self.has_value.insert(cursor.node_pos, false);
                } else {
                    cursor = self.child(cursor.bit_pos, child_number);
                }
            };
        }

        let value_index = self.has_value.rank1(cursor.node_pos);
        if self.has_value.get_bit(cursor.node_pos) {
            Some(self.values.set(value_index, value))
        } else {
            self.has_value.set_bit(cursor.node_pos, true);
            self.values.insert(value_index, value);
            None
        }
    }

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

            let (child_number, found) =
                self.bytes.child_number(byte_begin, degree, *byte);
            if !found {
                return None;
            }
            cursor = self.child(cursor.bit_pos, child_number);
        }

        if self.has_value.get_bit(cursor.node_pos) {
            let value_pos = self.has_value.rank1(cursor.node_pos);
            Some(self.values.get(value_pos))
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
        !self.trie.get_bit(cursor)
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

impl<T, K> FromIterator<(K, T)> for LoudsTrie<T>
where
    K: AsRef<[u8]>,
{
    fn from_iter<I>(input: I) -> Self
    where
        I: IntoIterator<Item = (K, T)>,
    {
        let mut trie = LoudsTrie::new();
        for (key, value) in input.into_iter() {
            trie.insert(key.as_ref(), value);
        }
        trie
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_louds_insertion() {
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
        let mut louds = LoudsTrie::new();
        for k in keys.iter() {
            louds.insert(k, k[k.len() - 1]);
        }

        assert_eq!(
            louds.has_value.to_vec(),
            vec![
                false, true, false, false, // root, b, c, d
                false, true, true, false, true, false, true, // level 2
                true, true, true, true, true, true,
            ]
        );
        assert_eq!(
            louds.trie.to_vec(),
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
        assert_eq!(
            louds.bytes.to_vec(),
            vec![
                b'b', b'c', b'd', b'e', b'f', b'g', b'h', b'i', b'j', b'k',
                b'l', b'm', b'n', b'o', b'p', b'q'
            ]
        );

        assert_eq!(
            louds.values.to_vec(),
            vec![
                &b'b', &b'f', &b'g', &b'i', &b'k', &b'l', &b'm', &b'n', &b'o',
                &b'p', &b'q'
            ]
        );
    }

    #[test]
    #[rustfmt::skip]
    fn test_louds_traverse() {
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
        let louds = LoudsTrie::from_iter(keys.iter().map(|k| (k, k[0])));

        assert_eq!(louds.degree(root.bit_pos), 3);

        let b = louds.child(root.bit_pos, 0);
        assert_eq!(b, Cursor { bit_pos: 4, node_pos: 1});
        assert_eq!(louds.degree(b.bit_pos), 2);

        let c = louds.child(root.bit_pos, 1);
        assert_eq!(c, Cursor { bit_pos: 7, node_pos: 2});
        assert_eq!(louds.degree(c.bit_pos), 1);

        let d = louds.child(root.bit_pos, 2);
        assert_eq!(d, Cursor { bit_pos: 9, node_pos: 3});
        assert_eq!(louds.degree(d.bit_pos), 4);

        let e = louds.child(b.bit_pos, 0);
        assert_eq!(e, Cursor { bit_pos: 14, node_pos: 4 });
        assert_eq!(louds.degree(e.bit_pos), 3);

        let f = louds.child(b.bit_pos, 1);
        assert_eq!(f, Cursor { bit_pos: 18, node_pos: 5 });
        assert_eq!(louds.degree(f.bit_pos), 0);

        let g = louds.child(c.bit_pos, 0);
        assert_eq!(g, Cursor { bit_pos: 19, node_pos: 6 });
        assert_eq!(louds.degree(g.bit_pos), 0);

        let h = louds.child(d.bit_pos, 0);
        assert_eq!(h, Cursor { bit_pos: 20, node_pos: 7 });
        assert_eq!(louds.degree(h.bit_pos), 1);

        let i = louds.child(d.bit_pos, 1);
        assert_eq!(i, Cursor { bit_pos: 22, node_pos: 8 });
        assert_eq!(louds.degree(i.bit_pos), 0);

        let j = louds.child(d.bit_pos, 2);
        assert_eq!(j, Cursor { bit_pos: 23, node_pos: 9 });
        assert_eq!(louds.degree(j.bit_pos), 2);

        let k = louds.child(d.bit_pos, 3);
        assert_eq!(k, Cursor { bit_pos: 26, node_pos: 10 });
        assert_eq!(louds.degree(k.bit_pos), 0);

        let l = louds.child(e.bit_pos, 0);
        assert_eq!(l, Cursor { bit_pos: 27, node_pos: 11 });
        assert_eq!(louds.degree(l.bit_pos), 0);

        let m = louds.child(e.bit_pos, 1);
        assert_eq!(m, Cursor { bit_pos: 28, node_pos: 12 });
        assert_eq!(louds.degree(m.bit_pos), 0);

        let n = louds.child(e.bit_pos, 2);
        assert_eq!(n, Cursor { bit_pos: 29, node_pos: 13 });
        assert_eq!(louds.degree(n.bit_pos), 0);

        let o = louds.child(h.bit_pos, 0);
        assert_eq!(o, Cursor { bit_pos: 30, node_pos: 14 });
        assert_eq!(louds.degree(o.bit_pos), 0);

        let p = louds.child(j.bit_pos, 0);
        assert_eq!(p, Cursor { bit_pos: 31, node_pos: 15 });
        assert_eq!(louds.degree(p.bit_pos), 0);

        let q = louds.child(j.bit_pos, 1);
        assert_eq!(q, Cursor { bit_pos: 32, node_pos: 16 });
        assert_eq!(louds.degree(q.bit_pos), 0);
    }

    #[test]
    fn test_louds_get() {
        let keys: [&[u8]; 11] = [
            b"bel", b"bem", b"ben", b"bf", b"cg", b"dho", b"di", b"djp",
            b"djq", b"dk", b"b",
        ];
        let louds = LoudsTrie::from_iter(keys.iter().map(|k| (k, k[0])));

        for key in keys.iter() {
            assert_eq!(louds.get(key), Some(&key[0]));
        }

        assert_eq!(louds.get(b"belarus"), None);
        assert_eq!(louds.get(b"dh"), None);
        assert_eq!(louds.get(b"dj"), None);
    }

    #[test]
    fn test_louds_get_numbers() {
        let numbers: [u16; 25] = [
            9424, 12398, 54780, 51835, 63026, 8401, 63521, 49588, 14290, 60102,
            12443, 35584, 11924, 55247, 770, 20443, 1862, 11155, 25753, 7685,
            1900, 7743, 43659, 63103, 3614,
        ];

        let louds =
            LoudsTrie::from_iter(numbers.iter().map(|k| (k.to_be_bytes(), k)));

        for k in numbers.iter() {
            assert_eq!(louds.get(k.to_be_bytes()), Some(&k));
        }
    }
}
