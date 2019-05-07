use std::collections::BTreeMap;

use super::map::YFastMap;
use super::LevelSearchable;

pub struct YFastSet<K: LevelSearchable<BTreeMap<K, ()>>> {
    map: YFastMap<K, ()>,
}

impl<K: LevelSearchable<BTreeMap<K, ()>>> YFastSet<K> {
    pub fn new() -> YFastSet<K> {
        YFastSet {
            map: YFastMap::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Clear the set, removing all keys
    pub fn clear(&mut self) {
        self.map.clear();
    }

    /// Return a reference to the value corresponding to the key
    pub fn contains(&self, key: K) -> bool {
        self.map.contains_key(key)
    }

    /// Adds a value to the set.
    ///
    /// If the set does not have the value present, return True.
    /// Otherwise, return False.
    pub fn insert(&mut self, key: K) -> bool {
        self.map.insert(key, ()).is_none()
    }

    /// Remove a value to the set.
    ///
    /// Returns whether the key is in the set
    pub fn remove(&mut self, key: K) -> bool {
        self.map.remove(key).is_some()
    }

    pub fn iter(&self) -> impl Iterator<Item = K> + '_ {
        self.map.iter().map(|k| k.0)
    }

    pub fn predecessor(&self, key: K) -> Option<K> {
        self.map.predecessor(key).map(|x| x.0)
    }

    pub fn successor(&self, key: K) -> Option<K> {
        self.map.successor(key).map(|x| x.0)
    }
}
