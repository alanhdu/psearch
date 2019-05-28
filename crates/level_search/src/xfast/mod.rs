mod map;
mod set;

pub use map::XFastMap;
pub use set::XFastSet;

pub trait LevelSearchable<T>: crate::level_search::LevelSearchable<T> {}
impl<T> LevelSearchable<T> for u32 {}
impl<T> LevelSearchable<T> for u64 {}
