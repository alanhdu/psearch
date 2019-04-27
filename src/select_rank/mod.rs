mod bits256;
mod bitvec;
mod sbitvec;
mod u64;

pub use bits256::Bits256;
pub use bitvec::BitVec;
pub use sbitvec::SBitVec;

pub trait SelectRank {
    /// Return the ith bit
    fn get_bit(&self, i: usize) -> bool;

    /// Return the number of 0s before the `i`th position
    fn rank0(&self, i: usize) -> usize;

    /// Return the number of 1s before the `i`th position
    fn rank1(&self, i: usize) -> usize;

    /// Return the position of the `i`th 0 (0-indexed)
    fn select0(&self, i: usize) -> usize;

    /// Return the position of the `i`th 1 (0-indexed)
    fn select1(&self, i: usize) -> usize;
}
