use std::iter::FromIterator;

use bit_parallel::{u16x32, u9x7::u9x7};

use super::SelectRank;
use crate::utils::binary_search_rank;

/// Static bit-vectors that support select and rank
#[derive(Debug, Eq, PartialEq)]
pub struct SBitVec {
    len: usize,
    blocks: Vec<u64>,
    index1: Vec<u9x7>,
    index2: Vec<[u16; 32]>,
    index3: Vec<u32>,
}

impl SBitVec {
    pub fn total_size(&self) -> usize {
        std::mem::size_of::<Self>()
            + 8 * self.blocks.len()
            + 8 * self.index1.len()
            + 2 * 32 * self.index2.len()
            + 4 * self.index3.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.len()).map(move |i| {
            let block_index = i / 64;
            let bit_index = i % 64;
            self.blocks[block_index] & (1 << bit_index) != 0
        })
    }
}

impl SelectRank for SBitVec {
    fn get_bit(&self, index: usize) -> bool {
        self.blocks[index / 64] & (1 << (index % 64)) != 0
    }

    /// Return the number of 0s before the `i`th position
    fn rank0(&self, index: usize) -> usize {
        index - self.rank1(index)
    }

    /// Return the number of 0s before the `i`th position
    fn rank1(&self, index: usize) -> usize {
        assert!(index < self.len);
        let block_rank = index / 64;
        let (i1, rem1) = (block_rank / 8, block_rank % 8);
        let (i2, rem2) = (i1 / 33, i1 % 33);

        let a = if i2 == 0 { 0 } else { self.index3[i2 - 1] };
        let b = if rem2 == 0 {
            0
        } else {
            self.index2[i2][rem2 - 1]
        };
        let c = if rem1 == 0 {
            0
        } else {
            self.index1[i1].get(rem1 - 1)
        };

        (a as usize)
            + (b as usize)
            + (c as usize)
            + self.blocks[block_rank].rank1(index % 64)
    }

    /// Return the position of the `i`th 1 (0-indexed)
    fn select0(&self, mut index: usize) -> usize {
        let index2_rank =
            binary_search_rank(index + 1, self.index3.len(), |mid| {
                (1 + mid) * (512 * 33) - self.index3[mid] as usize
            });
        index -= if index2_rank == 0 {
            0
        } else {
            512 * 33 * index2_rank - self.index3[index2_rank - 1] as usize
        };

        let index2 = &self.index2[index2_rank];
        let index1_rank = u16x32::rank_diff(
            &[
                512, 1024, 1536, 2048, 2560, 3072, 3584, 4096, 4608, 5120,
                5632, 6144, 6656, 7168, 7680, 8192, 8704, 9216, 9728, 10240,
                10752, 11264, 11776, 12288, 12800, 13312, 13824, 14336, 14848,
                15360, 15872, 16384,
            ],
            index2,
            1 + index as u16,
        );
        index -= if index1_rank == 0 {
            0
        } else {
            512 * index1_rank - index2[index1_rank - 1] as usize
        };

        let index1 = &self.index1[index2_rank * 33 + index1_rank];
        let block_rank = index1.rank_zero(index + 1);
        index -= if block_rank == 0 {
            0
        } else {
            64 * block_rank - index1.get(block_rank - 1) as usize
        };

        let block =
            self.blocks[block_rank + index1_rank * 8 + index2_rank * 8 * 33];
        let bit_rank = block.select0(index);

        bit_rank
            + block_rank * 64
            + index1_rank * 512
            + index2_rank * (512 * 33)
    }
    /// Return the position of the `i`th 1 (0-indexed)
    fn select1(&self, mut index: usize) -> usize {
        let index2_rank =
            binary_search_rank(1 + index, self.index3.len(), |mid| {
                self.index3[mid] as usize
            });
        index -= if index2_rank == 0 {
            0
        } else {
            self.index3[index2_rank - 1] as usize
        };

        let index2 = &self.index2[index2_rank];
        let index1_rank = u16x32::rank(index2, 1 + index as u16);
        index -= if index1_rank == 0 {
            0
        } else {
            index2[index1_rank - 1] as usize
        };

        let index1 = &self.index1[index2_rank * 33 + index1_rank];
        let block_rank = index1.rank(index + 1);
        index -= if block_rank == 0 {
            0
        } else {
            index1.get(block_rank - 1) as usize
        };

        let block =
            self.blocks[block_rank + index1_rank * 8 + index2_rank * 8 * 33];

        let bit_rank = block.select1(index);

        bit_rank
            + block_rank * 64
            + index1_rank * 512
            + index2_rank * (512 * 33)
    }
}

impl FromIterator<bool> for SBitVec {
    fn from_iter<T>(input: T) -> Self
    where
        T: IntoIterator<Item = bool>,
    {
        let iter = input.into_iter();
        let lower_bound_size = iter.size_hint().0;

        let mut sbitvec = SBitVec {
            len: 0,
            blocks: Vec::with_capacity(lower_bound_size),
            index1: Vec::with_capacity(lower_bound_size / (4 * 64)),
            index2: Vec::with_capacity(lower_bound_size / (4 * 64 * 32)),
            index3: Vec::with_capacity(lower_bound_size / (4 * 64 * 32)),
        };

        let mut block = 0u64;
        let mut index1 = [0u16; 8];
        let mut index2 = [0u16; 33];
        let mut index3 = 0;

        for (i, bit) in iter.enumerate() {
            sbitvec.len = i;
            if i > 0 {
                if i % 64 == 0 {
                    sbitvec.blocks.push(block);
                    index1[(i / 64 - 1) % 8] = block.count_ones() as u16;
                    block = 0;
                }
                if i % 512 == 0 {
                    let mut output = [0; 7];
                    output[0] = index1[0];
                    for i in 1..7 {
                        output[i] = output[i - 1] + index1[i];
                    }
                    sbitvec.index1.push(u9x7::new(output));
                    index2[(i / 512 - 1) % 33] = output[6] + index1[7];
                    index1 = [0; 8];
                }

                if i % (512 * 33) == 0 {
                    let mut output = [0; 32];
                    output[0] = index2[0];
                    for i in 1..32 {
                        output[i] = output[i - 1] + index2[i];
                    }
                    sbitvec.index2.push(output);

                    index3 += u32::from(output[31]) + u32::from(index2[32]);
                    sbitvec.index3.push(index3);

                    index2 = [0; 33];
                }
            }

            if bit {
                block |= 1 << (i % 64);
            }
        }
        sbitvec.len += 1;

        // Handle stragglers:
        let i = sbitvec.len - 1;
        sbitvec.blocks.push(block);
        index1[(i / 64) % 8] = block.count_ones() as u16;

        let mut output = [0; 7];
        output[0] = index1[0];
        for i in 1..7 {
            output[i] = output[i - 1] + index1[i];
        }
        sbitvec.index1.push(u9x7::new(output));

        index2[(i / 512) % 33] = output[6] + index1[7];

        let mut output = [0; 32];
        output[0] = index2[0];
        for i in 1..32 {
            output[i] = output[i - 1] + index2[i];
        }
        sbitvec.index2.push(output);

        index3 += u32::from(output[31]) + u32::from(index2[32]);
        sbitvec.index3.push(index3);

        sbitvec
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sbitvec_total_size() {
        // Theoretically, if we store n bits we need:
        //  - blocks: n bits
        //  - index1: 8 bits  / 64 bits       = 1/8 n
        //  - index2: 16 bits / 512 bits      = 1 / 32 n
        //  - index3: 32 bits / (512*33) bits = 1 / 528 n
        //
        // which should be about 1.158144n bits to store.
        let bits = SBitVec::from_iter(vec![true; 80]);
        // 196 bytes to store 10 bytes of data
        //  = 19.6 x overhead
        assert_eq!(bits.total_size(), 196);

        let bits = SBitVec::from_iter(vec![false; 80000]);
        // 11700 bytes to store 10000 bytes of data
        //  = 1.17x overhead
        assert_eq!(bits.total_size(), 11700);

        let bits = SBitVec::from_iter(vec![false; 800000]);
        // 115872 bytes to store 100000 bytes of data
        //  = 1.16x overhead
        assert_eq!(bits.total_size(), 115872);
    }

    #[test]
    fn test_sbitvec_from_iter_small() {
        let items = vec![true, false, false, true, true, false];
        let bits = SBitVec::from_iter(items);

        assert_eq!(bits.len, 6);
        assert_eq!(bits.blocks, vec![0b011001]);
        assert_eq!(bits.index1, vec![u9x7::new([3; 7])]);
        assert_eq!(bits.index2, vec![[3; 32]]);
        assert_eq!(bits.index3, vec![3]);
    }

    #[test]
    fn test_sbitvec_from_iter_block_size() {
        let items = vec![true; 64];
        let bits = SBitVec::from_iter(items.iter().cloned());

        assert_eq!(bits.len, items.len());
        assert_eq!(bits.blocks, vec![u64::max_value()]);
        assert_eq!(bits.index1, vec![u9x7::new([64; 7])]);
        assert_eq!(bits.index2, vec![[64; 32]]);
        assert_eq!(bits.index3, vec![64]);
    }

    #[test]
    fn test_sbitvec_from_iter_blocks() {
        let bits = SBitVec::from_iter(
            vec![false; 10000].into_iter().chain(vec![true; 10000]),
        );

        assert_eq!(bits.len, 20000);

        // 10000 / 64 = 156 remainder 16
        let mut expected = vec![0; 156];
        expected.push(u64::max_value() << 16);
        expected.append(&mut vec![u64::max_value(); 155]);
        expected.push(u64::max_value() >> 32);
        assert_eq!(bits.blocks, expected);

        // 10000 / 512 = 19 remainder 272
        // 272 / 64 = 4 remainder 16
        let mut expected = vec![u9x7::new([0; 7]); 19];
        expected.push(u9x7::new([0, 0, 0, 0, 48, 48 + 64, 48 + 2 * 64]));
        expected.append(&mut vec![
            u9x7::new([64, 128, 192, 256, 320, 384, 448]);
            19
        ]);
        // 20000 % 512 = 32
        expected.push(u9x7::new([32; 7]));
        assert_eq!(bits.index1, expected);

        // 10000 / (512 * 33) = 0 rem 10000
        // 20000 / (512 * 33) = 1 rem 3104
        assert_eq!(
            bits.index2,
            vec![
                [
                    // 19 0s, then 240 = 48 + 3 * 64 ,then increment by 512
                    0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                    240, 752, 1264, 1776, 2288, 2800, 3312, 3824, 4336, 4848,
                    5360, 5872, 6384,
                ],
                [
                    // Increment by 512 up until 3104, then stay there
                    512, 1024, 1536, 2048, 2560, 3072, 3104, 3104, 3104, 3104,
                    3104, 3104, 3104, 3104, 3104, 3104, 3104, 3104, 3104, 3104,
                    3104, 3104, 3104, 3104, 3104, 3104, 3104, 3104, 3104, 3104,
                    3104, 3104
                ]
            ]
        );
        assert_eq!(bits.index3, vec![6384 + 512, 10000]);
    }

    #[test]
    fn test_sbitvec_select_rank_blocks() {
        let bits = SBitVec::from_iter(
            vec![false; 20000].into_iter().chain(vec![true; 20000]),
        );

        for i in 0..20000 {
            assert_eq!(bits.rank0(i), i);
            assert_eq!(bits.rank1(i), 0);
        }
        for i in 20000..40000 {
            assert_eq!(bits.rank0(i), 20000);
            assert_eq!(bits.rank1(i), i - 20000);
        }

        for i in 0..20000 {
            assert_eq!(bits.select0(i), i);
            assert_eq!(bits.select1(i), 20000 + i);
        }
    }

    #[test]
    fn test_sbitvec_select_rank_alternating() {
        let mut bits = vec![true; 40000];
        for i in 0..20000 {
            bits[i * 2] = false;
        }
        let bits = SBitVec::from_iter(bits);

        for i in 0..40000 {
            assert_eq!(bits.rank0(i), (i + 1) / 2);
            assert_eq!(bits.rank1(i), i / 2);
        }

        for i in 0..20000 {
            assert_eq!(bits.select0(i), 2 * i);
            assert_eq!(bits.select1(i), 2 * i + 1);
        }
    }

    #[test]
    fn test_sbitvec_boundary_construction() {
        assert_eq!(SBitVec::from_iter(vec![false; 64]).blocks, vec![0]);
        assert_eq!(SBitVec::from_iter(vec![false; 65]).blocks, vec![0, 0]);
    }
}
