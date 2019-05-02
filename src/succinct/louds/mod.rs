#![allow(dead_code)]

use crate::select_rank::{BitVec, SelectRank};

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

    pub fn insert<K: AsRef<[u8]>>(&mut self, key: K, value: T) -> Option<T> {
        dbg!(key.as_ref());
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
                if found != byte {
                    let child =
                        self.trie.select0(cursor.node_pos + child_number);
                    self.trie.insert(child, false);
                    self.trie.insert(cursor.bit_pos, true);
                    self.bytes.insert(byte_begin + child_number, byte);

                    cursor = self.child(cursor.bit_pos, child_number);
                    self.has_value.insert(cursor.node_pos, false);
                } else {
                    cursor = self.child(cursor.bit_pos, child_number);
                }
            };

            eprintln!();
            #[cfg(test)]
            dbg!(
                &cursor,
                byte,
                self.bytes.to_vec(),
                self.has_value.to_vec(),
                self.trie.to_vec()
            );

            eprintln!();
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
            if found != *byte {
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
        // let keys: [&[u8]; 11] = [
        //     b"bel", b"bem", b"ben", b"bf", b"cg", b"dho", b"di", b"djp",
        //     b"djq", b"dk", b"b",
        // ];
        let keys: [&[u8]; 3] = [b"bel", b"cg", b"bem"];
        let mut louds = LoudsTrie::new();
        for k in keys.iter() {
            louds.insert(k, k[k.len() - 1]);
        }

        assert_eq!(
            louds.has_value.to_vec(),
            vec![false, false, false, false, true, true, true]
        );
        assert_eq!(louds.values.to_vec(), vec![&b'g', &b'l', &b'm']);
        assert_eq!(
            louds.bytes.to_vec(),
            vec![b'b', b'c', b'e', b'g', b'l', b'm']
        );
        assert_eq!(
            louds.trie.to_vec(),
            vec![
                true, true, false, true, false, true, false, true, true, false,
                false, false, false
            ]
        );
        /*
        assert_eq!(
            louds.has_value.to_vec(),
            vec![
                false, false, false, false, // root, b, c, d
                false, true, true, false, true, false, false, // level 2
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
        */
    }
}
