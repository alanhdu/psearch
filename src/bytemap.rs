use std::fmt::Debug;

// TODO(alan): SIMD-ify
#[derive(Debug, Eq, PartialEq)]
pub struct ByteMap<T> {
    len: u16,
    node: Node<T>,
}

impl<T> Default for ByteMap<T> {
    fn default() -> ByteMap<T> {
        ByteMap::new()
    }
}

#[derive(Debug, Eq, PartialEq)]
enum Node<T> {
    N4(Box<Node4<T>>),
    N16(Box<Node16<T>>),
    N48(Box<Node48<T>>),
    N256(Box<Node256<T>>),
}

impl<T> Node<T> {
    fn upsize(&mut self) {
        match self {
            Node::N4(ref mut n) => {
                let mut new = Box::new(Node16 {
                    bytes: [0; 16],
                    values: Default::default(),
                });
                new.bytes[0] = n.bytes[0];
                new.bytes[1] = n.bytes[1];
                new.bytes[2] = n.bytes[2];
                new.bytes[3] = n.bytes[3];
                new.values[..4].swap_with_slice(&mut n.values);
                *self = Node::N16(new);
            }
            Node::N16(ref mut n) => {
                let mut new = Box::new(Node48 {
                    positions: [u8::max_value(); 256],
                    values: unsafe { std::mem::zeroed() },
                });

                for i in 0..16 {
                    new.positions[n.bytes[i as usize] as usize] = i;
                }
                new.values[..16].swap_with_slice(&mut n.values);
                *self = Node::N48(new);
            }
            Node::N48(ref mut n) => {
                let mut new = Box::new(Node256 {
                    values: unsafe { std::mem::zeroed() },
                });

                for i in 0..=255 {
                    if n.positions[i] < 48 {
                        new.values[i as usize] = std::mem::replace(
                            &mut n.values[n.positions[i] as usize],
                            None,
                        );
                    }
                }
                *self = Node::N256(new);
            }
            _ => {}
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Node4<T> {
    bytes: [u8; 4],
    values: [Option<T>; 4],
}

#[derive(Debug, Eq, PartialEq)]
struct Node16<T> {
    bytes: [u8; 16],
    values: [Option<T>; 16],
}

struct Node48<T> {
    positions: [u8; 256],
    values: [Option<T>; 48],
}

impl<T: Debug> Debug for Node48<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Node48")
            .field("positions", &(&self.positions as &[u8]))
            .field("values", &(&self.values as &[Option<T>]))
            .finish()
    }
}
impl<T: Eq> Eq for Node48<T> {}
impl<T: PartialEq> PartialEq for Node48<T> {
    fn eq(&self, other: &Self) -> bool {
        for (a, b) in
            Iterator::zip(self.positions.iter(), other.positions.iter())
        {
            if a != b {
                return false;
            }
        }
        for (a, b) in Iterator::zip(self.values.iter(), other.values.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }
}

struct Node256<T> {
    values: [Option<T>; 256],
}

impl<T: Debug> Debug for Node256<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.debug_struct("Node256")
            .field("values", &(&self.values as &[Option<T>]))
            .finish()
    }
}
impl<T: Eq> Eq for Node256<T> {}
impl<T: PartialEq> PartialEq for Node256<T> {
    fn eq(&self, other: &Self) -> bool {
        for (a, b) in Iterator::zip(self.values.iter(), other.values.iter()) {
            if a != b {
                return false;
            }
        }
        true
    }
}

pub enum Entry<'a, T> {
    Occupied(OccupiedEntry<'a, T>),
    Vacant(VacantEntry<'a, T>),
}

pub struct OccupiedEntry<'a, T> {
    map: &'a mut ByteMap<T>,
    key: u8,
    rank: usize,
}

impl<'a, T> OccupiedEntry<'a, T> {
    pub fn get_mut(&mut self) -> &mut T {
        match self.map.node {
            Node::N4(ref mut n) => n.values[self.rank].as_mut().unwrap(),
            Node::N16(ref mut n) => n.values[self.rank].as_mut().unwrap(),
            Node::N48(ref mut n) => n.values
                [n.positions[self.key as usize] as usize]
                .as_mut()
                .unwrap(),
            Node::N256(ref mut n) => {
                n.values[self.key as usize].as_mut().unwrap()
            }
        }
    }

    pub fn remove(&mut self) {
        let len = self.map.len;
        match self.map.node {
            Node::N4(ref mut n) => {
                for i in self.rank..(n.bytes.len() - 1) {
                    n.bytes[i] = n.bytes[i + 1];
                    n.values.swap(i, i + 1);
                }
                n.bytes[n.bytes.len() - 1] = 0;
                n.values[n.bytes.len() - 1] = None;
            }
            Node::N16(ref mut n) => {
                for i in self.rank..(n.bytes.len() - 1) {
                    n.bytes[i] = n.bytes[i + 1];
                    n.values.swap(i, i + 1);
                }
                n.bytes[n.bytes.len() - 1] = 0;
                n.values[n.bytes.len() - 1] = None;
            }
            Node::N48(ref mut n) => {
                let pos = std::mem::replace(
                    &mut n.positions[self.key as usize],
                    0xFF,
                );
                if pos != (len - 1) as u8 {
                    let last = n
                        .positions
                        .iter()
                        .position(|p| *p == (len - 1) as u8)
                        .unwrap();
                    n.values.swap(n.positions[last] as usize, pos as usize);
                    n.positions[last] = pos;
                }
                n.values[(len - 1) as usize] = None;
            }
            Node::N256(ref mut n) => {
                n.values[self.key as usize] = None;
            }
        }
        self.map.len -= 1;
    }
}

pub struct VacantEntry<'a, T> {
    map: &'a mut ByteMap<T>,
    key: u8,
    rank: usize,
}

impl<'a, T> VacantEntry<'a, T> {
    pub fn insert(&mut self, value: T) {
        match self.map.node {
            Node::N4(ref mut n) => {
                for i in ((self.rank + 1)..n.bytes.len()).rev() {
                    n.bytes[i] = n.bytes[i - 1];
                    n.values.swap(i, i - 1);
                }
                n.bytes[self.rank] = self.key;
                n.values[self.rank] = Some(value);
            }
            Node::N16(ref mut n) => {
                for i in ((self.rank + 1)..n.bytes.len()).rev() {
                    n.bytes[i] = n.bytes[i - 1];
                    n.values.swap(i, i - 1);
                }
                n.bytes[self.rank] = self.key;
                n.values[self.rank] = Some(value);
            }
            Node::N48(ref mut n) => {
                n.positions[self.key as usize] = self.map.len as u8;
                n.values[self.map.len as usize] = Some(value);
            }
            Node::N256(ref mut n) => {
                n.values[self.key as usize] = Some(value);
            }
        }
        self.map.len += 1;
    }
}

impl<T> ByteMap<T> {
    pub fn new() -> ByteMap<T> {
        ByteMap {
            node: Node::N4(Box::new(Node4 {
                bytes: [0; 4],
                values: Default::default(),
            })),
            len: 0,
        }
    }

    #[cfg(test)]
    pub(crate) fn from_vec(vec: Vec<(u8, T)>) -> ByteMap<T> {
        let mut map = ByteMap::new();
        for (key, value) in vec.into_iter() {
            map.insert(key, value);
        }
        map
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn predecessor(&self, byte: u8) -> Option<(u8, &T)> {
        if self.is_empty() {
            return None;
        }
        let len = self.len as usize;
        match self.node {
            Node::N4(ref n) => {
                for (i, b) in n.bytes[..len].iter().cloned().enumerate().rev() {
                    if b <= byte {
                        return n.values[i].as_ref().map(|r| (b, r));
                    }
                }
            }
            Node::N16(ref n) => {
                for (i, b) in n.bytes[..len].iter().cloned().enumerate().rev() {
                    if b <= byte {
                        return n.values[i].as_ref().map(|r| (b, r));
                    }
                }
            }
            Node::N48(ref n) => {
                for b in (0..=byte).rev() {
                    let pos = n.positions[b as usize] as usize;
                    if pos < 48 {
                        return n.values[pos].as_ref().map(|r| (b, r));
                    }
                }
            }
            Node::N256(ref n) => {
                for b in (0..=byte).rev() {
                    if n.values[b as usize].is_some() {
                        return n.values[b as usize].as_ref().map(|r| (b, r));
                    }
                }
            }
        }
        None
    }

    pub fn predecessor_mut(&mut self, byte: u8) -> Option<(u8, &mut T)> {
        if self.is_empty() {
            return None;
        }
        let len = self.len as usize;
        match self.node {
            Node::N4(ref mut n) => {
                for (i, b) in n.bytes[..len].iter().cloned().enumerate().rev() {
                    if b <= byte {
                        return n.values[i].as_mut().map(|r| (b, r));
                    }
                }
            }
            Node::N16(ref mut n) => {
                for (i, b) in n.bytes[..len].iter().cloned().enumerate().rev() {
                    if b <= byte {
                        return n.values[i].as_mut().map(|r| (b, r));
                    }
                }
            }
            Node::N48(ref mut n) => {
                for b in (0..=byte).rev() {
                    let pos = n.positions[b as usize] as usize;
                    if pos < 48 {
                        return n.values[pos].as_mut().map(|r| (b, r));
                    }
                }
            }
            Node::N256(ref mut n) => {
                for b in (0..=byte).rev() {
                    if n.values[b as usize].is_some() {
                        return n.values[b as usize].as_mut().map(|r| (b, r));
                    }
                }
            }
        }
        None
    }

    pub fn successor(&self, byte: u8) -> Option<(u8, &T)> {
        if self.is_empty() {
            return None;
        }
        match self.node {
            Node::N4(ref n) => {
                for (i, b) in n.bytes.iter().cloned().enumerate() {
                    if b >= byte {
                        return n.values[i].as_ref().map(|r| (b, r));
                    }
                }
            }
            Node::N16(ref n) => {
                for (i, b) in n.bytes.iter().cloned().enumerate() {
                    if b >= byte {
                        return n.values[i].as_ref().map(|r| (b, r));
                    }
                }
            }
            Node::N48(ref n) => {
                for b in byte..=255 {
                    let pos = n.positions[b as usize] as usize;
                    if pos < 48 {
                        return n.values[pos].as_ref().map(|r| (b, r));
                    }
                }
            }
            Node::N256(ref n) => {
                for b in byte..=255 {
                    if n.values[b as usize].is_some() {
                        return n.values[b as usize].as_ref().map(|r| (b, r));
                    }
                }
            }
        }
        None
    }

    pub fn successor_mut(&mut self, byte: u8) -> Option<(u8, &mut T)> {
        if self.is_empty() {
            return None;
        }
        match self.node {
            Node::N4(ref mut n) => {
                for (i, b) in n.bytes.iter().cloned().enumerate() {
                    if b >= byte {
                        return n.values[i].as_mut().map(|r| (b, r));
                    }
                }
            }
            Node::N16(ref mut n) => {
                for (i, b) in n.bytes.iter().cloned().enumerate() {
                    if b >= byte {
                        return n.values[i].as_mut().map(|r| (b, r));
                    }
                }
            }
            Node::N48(ref mut n) => {
                for b in byte..=255 {
                    let pos = n.positions[b as usize] as usize;
                    if pos < 48 {
                        return n.values[pos].as_mut().map(|r| (b, r));
                    }
                }
            }
            Node::N256(ref mut n) => {
                for b in byte..=255 {
                    if n.values[b as usize].is_some() {
                        return n.values[b as usize].as_mut().map(|r| (b, r));
                    }
                }
            }
        }
        None
    }

    pub fn insert(&mut self, key: u8, value: T) -> Option<T> {
        match self.entry(key) {
            Entry::Vacant(mut v) => {
                v.insert(value);
                None
            }
            Entry::Occupied(mut o) => {
                Some(std::mem::replace(o.get_mut(), value))
            }
        }
    }

    pub fn get(&self, key: u8) -> Option<&T> {
        let len = self.len as usize;
        match self.node {
            Node::N4(ref n) => {
                for (i, byte) in n.bytes[..len].iter().cloned().enumerate() {
                    if byte == key {
                        return n.values[i].as_ref();
                    }
                }
                None
            }
            Node::N16(ref n) => {
                for (i, byte) in n.bytes[..len].iter().cloned().enumerate() {
                    if byte == key {
                        return n.values[i].as_ref();
                    }
                }
                None
            }
            Node::N48(ref n) => {
                n.values[n.positions[key as usize] as usize].as_ref()
            }
            Node::N256(ref n) => n.values[key as usize].as_ref(),
        }
    }

    pub fn get_mut(&mut self, key: u8) -> Option<&mut T> {
        let len = self.len as usize;
        match self.node {
            Node::N4(ref mut n) => {
                for (i, byte) in n.bytes[..len].iter().cloned().enumerate() {
                    if byte == key {
                        return n.values[i].as_mut();
                    }
                }
                None
            }
            Node::N16(ref mut n) => {
                for (i, byte) in n.bytes[..len].iter().cloned().enumerate() {
                    if byte == key {
                        return n.values[i].as_mut();
                    }
                }
                None
            }
            Node::N48(ref mut n) => {
                n.values[n.positions[key as usize] as usize].as_mut()
            }
            Node::N256(ref mut n) => n.values[key as usize].as_mut(),
        }
    }

    pub fn entry(&mut self, key: u8) -> Entry<'_, T> {
        // Resize if necessary
        match (&self.node, self.len) {
            (Node::N4(_), 4) => self.node.upsize(),
            (Node::N16(_), 16) => self.node.upsize(),
            (Node::N48(_), 48) => self.node.upsize(),
            _ => {}
        }

        match self.node {
            Node::N4(ref n) => {
                let mut rank = 0;
                for (i, byte) in
                    n.bytes.iter().cloned().take(self.len as usize).enumerate()
                {
                    if byte == key {
                        return Entry::Occupied(OccupiedEntry {
                            map: self,
                            key,
                            rank: i,
                        });
                    } else if byte > key {
                        break;
                    }
                    rank += 1;
                }
                return Entry::Vacant(VacantEntry {
                    map: self,
                    key,
                    rank,
                });
            }
            Node::N16(ref n) => {
                let mut rank = 0;
                for (i, byte) in
                    n.bytes.iter().cloned().take(self.len as usize).enumerate()
                {
                    if byte == key {
                        return Entry::Occupied(OccupiedEntry {
                            map: self,
                            key,
                            rank: i,
                        });
                    } else if byte > key {
                        break;
                    }
                    rank += 1;
                }

                Entry::Vacant(VacantEntry {
                    map: self,
                    key,
                    rank,
                })
            }
            Node::N48(ref n) => {
                if n.positions[key as usize] < 48 {
                    Entry::Occupied(OccupiedEntry {
                        map: self,
                        key,
                        rank: 0,
                    })
                } else {
                    Entry::Vacant(VacantEntry {
                        map: self,
                        key,
                        rank: 0,
                    })
                }
            }
            Node::N256(ref n) => {
                if n.values[key as usize].is_some() {
                    Entry::Occupied(OccupiedEntry {
                        map: self,
                        key,
                        rank: 0,
                    })
                } else {
                    Entry::Vacant(VacantEntry {
                        map: self,
                        key,
                        rank: 0,
                    })
                }
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_bytemap_insert() {
        let keys: Vec<u8> =
            vec![38, 0, 1, 39, 2, 40, 3, 4, 5, 6, 86, 7, 8, 9, 10, 11, 0];

        let mut map = ByteMap::new();
        let mut expected = BTreeSet::new();
        for key in keys.iter().cloned() {
            assert_eq!(map.insert(key, key).is_some(), !expected.insert(key));
            assert_eq!(map.successor(key), Some((key, &key)));
            assert_eq!(map.predecessor(key), Some((key, &key)));
            assert_eq!(map.len as usize, expected.len());

            for i in 0..=255 {
                assert_eq!(
                    map.successor(i),
                    expected.range(i..).next().map(|k| (*k, k))
                );
                assert_eq!(
                    map.successor_mut(i).map(|(k, v)| (k, *v)),
                    expected.range(i..).next().map(|k| (*k, *k))
                );
                assert_eq!(
                    map.predecessor(i),
                    expected.range(0..=i).next_back().map(|k| (*k, k))
                );
                assert_eq!(
                    map.predecessor_mut(i).map(|(k, v)| (k, *v)),
                    expected.range(0..=i).next_back().map(|k| (*k, *k))
                );
            }
        }
    }
}
