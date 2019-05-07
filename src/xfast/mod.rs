mod map;
mod set;
mod traits;

pub use map::XFastMap;
pub use set::XFastSet;

pub trait LevelSearchable<T>: traits::LevelSearchable<T> {}
impl<T> LevelSearchable<T> for u32 {}
