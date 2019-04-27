use std::collections::{BTreeMap, VecDeque};
use std::iter::FromIterator;

use crate::select_rank::{SBitVec, SelectRank};

/// A Static LOUDS trie
pub struct SLouds<T> {
    trie: SBitVec,
    has_value: SBitVec,
    bytes: Vec<u8>,
    values: Vec<T>,
}

struct Cursor {
    bit_pos: usize,
    node_pos: usize,
}

impl Cursor {
    fn from_bit_pos(trie: &SBitVec, pos: usize) -> Cursor {
        Cursor {
            bit_pos: pos,
            node_pos: trie.select0(pos) - (trie.get_bit(pos) == false) as usize,
        }
    }
}

impl<T> SLouds<T> {
    pub fn get<K: AsRef<[u8]>>(&self, key: K) -> Option<&T> {
        let mut cursor = Cursor {
            bit_pos: 0,
            node_pos: 0,
        };

        for byte in key.as_ref().iter() {
            let degree = self.degree(cursor.bit_pos);
            let slice = &self.bytes[cursor.node_pos..cursor.node_pos + degree];
            let child = slice.binary_search(byte).ok()?;

            cursor = self.child(cursor, child);
        }

        let value_pos = self.has_value.rank1(cursor.node_pos);
        self.values.get(value_pos)
    }

    /// Get the bit-index of `cursor`'s `i`th child
    fn child(&self, cursor: Cursor, i: usize) -> Cursor {
        Cursor::from_bit_pos(
            &self.trie,
            self.trie.select0(self.trie.rank1(cursor.bit_pos + i) - 1) + 1,
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

/// A really bad trie implementation to construct the SLouds trie
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

impl<T, K> FromIterator<(K, T)> for SLouds<T>
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

        SLouds {
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
        let slouds = SLouds::from_iter(keys.iter().map(|k| (k, ())));

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
}
