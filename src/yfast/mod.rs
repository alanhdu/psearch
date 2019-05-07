mod map;
mod tree;

pub use map::YFastMap;
pub trait LevelSearchable<T>: crate::level_search::LevelSearchable<T> {}
impl<T> LevelSearchable<T> for u32 {}
impl<T> LevelSearchable<T> for u64 {}

use std::collections::BTreeMap;
type LinkedBTree<K, V> = crate::level_search::LNode<K, BTreeMap<K, V>>;
