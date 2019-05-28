mod map;
mod set;
mod tree;

pub use map::YFastMap;
pub use set::YFastSet;
use tree::BTreeRange;

pub trait LevelSearchable<T>: crate::level_search::LevelSearchable<T> {}
impl<T> LevelSearchable<T> for u32 {}
impl<T> LevelSearchable<T> for u64 {}

type LinkedBTree<K, V> = crate::level_search::LNode<K, BTreeRange<K, V>>;
