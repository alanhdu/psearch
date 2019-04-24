const SIZE: u32 = 64;
const INCREMENT: [u32; 4] =
    [0x01_01_01_00, 0x01_01_00_00, 0x01_00_00_00, 0x00_00_00_00];

/// parallel deposit
fn pdep(src: u64, mask: u64) -> u64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::_pdep_u64;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::_pdep_u64;

    unsafe { _pdep_u64(src, mask) }
}

/// parallel extract
fn pext(src: u64, mask: u64) -> u64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::_pext_u64;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::_pext_u64;

    unsafe { _pext_u64(src, mask) }
}

/// A bitstring holding up to 256 bits
///
/// We will assume that the bits are "packed" -- in other words, that if
/// this bitstring contains n bits, then it will be the *first* n bits
/// in the bitstring
#[derive(Debug, Eq, PartialEq)]
pub struct Bits256 {
    ones: [u8; 4],
    len: u32, // NOTE: this actual fits in a u8, if we need more space
    /// Containers holding our actual bitstring. Within a u64, bits go
    /// from right to left (i.e. bit number 0 is the *least* significant
    /// bit). This allows for efficient implementation of SELECT using
    /// pdep
    bits: [u64; 4],
}

impl Bits256 {
    pub fn num_ones(&self) -> u32 {
        u32::from(self.ones[3]) + self.bits[3].count_ones()
    }

    pub fn num_zeros(&self) -> u32 {
        self.len - self.num_ones()
    }

    /// Insert a bit at our index
    pub fn insert_bit(&mut self, index: usize, bit: bool) {
        debug_assert!(index <= self.len as usize);

        let index = index as u8;
        let mut upper = (index >> 6) as usize;
        let lower = index & 0b0011_1111;

        let mut last = self.bits[upper] >> 63;
        // TODO(alan): See if you can SIMD accelerate this (shift right
        // + bitwise |)
        self.bits[upper] =
            pdep(self.bits[upper], !(1 << lower)) | ((bit as u64) * (1 << lower));

        self.ones = (u32::from_le_bytes(self.ones)
            + (bit as u32) * INCREMENT[upper]
            - (last as u32) * INCREMENT[upper])
            .to_le_bytes();

        self.len += 1;
        upper += 1;
        while upper * 64 < self.len as usize {
            let old = last;
            last = self.bits[upper] >> 63;
            self.ones = (u32::from_le_bytes(self.ones)
                + (last as u32) * INCREMENT[upper]
                - (old as u32) * INCREMENT[upper])
                .to_le_bytes();
            self.bits[upper] = (self.bits[upper] << 1) | (old as u64);
            upper += 1;
        }
    }

    /// Remove a bit at our index
    pub fn remove_bit(&mut self, index: usize) -> bool {
        debug_assert!(index < self.len as usize);

        // TODO(alan): See if you can SIMD accelerate this shift left

        let index = index as u8;
        let mut upper = (index >> 6) as usize;
        let lower = index & 0b0011_1111;

        let output = (self.bits[upper] & 1 << lower) != 0;

        self.bits[upper] = pext(self.bits[upper], !(1 << lower));
        self.ones = (u32::from_le_bytes(self.ones)
            - (output as u32) * INCREMENT[upper])
            .to_le_bytes();

        upper += 1;
        while upper * 64 < self.len as usize {
            let bit = self.bits[upper] & 0b1 != 0;
            self.bits[upper] >>= 1;
            self.bits[upper - 1] |= (bit as u64) << 63;

            debug_assert!(upper < 4); // If upper == 4, upper * 64 > self.len
                                      // Update *previous* version
            self.ones[upper] += bit as u8;
            upper += 1;
        }

        self.len -= 1;
        output
    }

    /// Set the bit at index to the given value
    pub fn set_bit(&mut self, index: usize, bit: bool) {
        debug_assert!(index < self.len as usize);
        let index = index as u8;

        let upper = (index >> 6) as usize;
        let lower = index & 0b0011_1111;

        let prev_bit = self.bits[upper] & (1 << lower) != 0;
        if prev_bit != bit {
            if bit {
                self.bits[upper] |= 1 << lower;
                self.ones = (u32::from_le_bytes(self.ones) + INCREMENT[upper])
                    .to_le_bytes();
            } else {
                self.bits[upper] &= !(1 << lower);
                self.ones = (u32::from_le_bytes(self.ones) - INCREMENT[upper])
                    .to_le_bytes();
            }
        }
    }

    /// Return the number of 0s before the `i`th position
    pub fn rank0(&self, index: u32) -> u32 {
        debug_assert!(index < self.len);
        index - self.rank1(index)
    }

    /// Return the number of 1s before the `i`th position
    pub fn rank1(&self, index: u32) -> u32 {
        debug_assert!(index < self.len);
        let upper = (index as u8) >> 6;
        let lower = (index as u8) & 0b0011_1111;

        let bits = if lower == 0 {
            0
        } else {
            (self.bits[upper as usize] << (SIZE - lower as u32)).count_ones()
        };
        bits + u32::from(self.ones[upper as usize])
    }

    /// Return the position of the `i`th 0 (0-indexed)
    pub fn select0(&self, index: u32) -> u32 {
        debug_assert!(index < 256);
        debug_assert!(index < self.len);
        let index = index as u8;

        let i = if index < 2 * 64 - self.ones[2] {
            if index < 64 - self.ones[1] {
                0
            } else {
                1
            }
        } else if index < 3 * 64 - self.ones[3] {
            2
        } else {
            3
        };

        let index = index - ((i as u8) * 64 - self.ones[i]);

        (i as u32) * 64 + pdep(1 << index, !self.bits[i]).trailing_zeros()
    }

    /// Return the position of the `i`th 1 (0-indexed)
    pub fn select1(&self, index: u32) -> u32 {
        debug_assert!(index < 256);
        debug_assert!(index < self.len);
        let index = index as u8;

        let i = if index < self.ones[2] {
            if index < self.ones[1] {
                0
            } else {
                1
            }
        } else if index < self.ones[3] {
            2
        } else {
            3
        };

        let index = index - self.ones[i];
        (i as u32) * 64 + pdep(1 << index, self.bits[i]).trailing_zeros()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn test_sizes() {
        assert_eq!(
            std::mem::size_of::<Bits256>(),
            5 * std::mem::size_of::<u64>()
        );
    }

    #[test]
    fn test_bits256_insert_bit() {
        let mut bits256 = Bits256 {
            ones: [0, 0, 0, 0],
            len: 63,
            bits: [0, 0, 0, 0],
        };

        bits256.insert_bit(0, true);
        bits256.insert_bit(25, true);
        bits256.insert_bit(25, false);
        bits256.insert_bit(25, true);
        bits256.insert_bit(64, true);

        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 3, 4, 4],
                len: 68,
                bits: [0b1 | 1 << 25 | 1 << 27, 1, 0, 0],
            }
        );
    }

    #[test]
    fn test_bits256_remove_bit() {
        let mut bits256 = Bits256 {
            ones: [0, 3, 5, 5],
            len: 68,
            bits: [0b1 | 1 << 25 | 1 << 27, 0b11, 0, 0],
        };

        assert_eq!(bits256.remove_bit(1), false);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 4, 5, 5],
                len: 67,
                bits: [0b1 | 1 << 24 | 1 << 26 | 1 << 63, 0b1, 0, 0],
            }
        );

        assert_eq!(bits256.remove_bit(64), true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 4, 4, 4],
                len: 66,
                bits: [0b1 | 1 << 24 | 1 << 26 | 1 << 63, 0, 0, 0],
            }
        );

        assert_eq!(bits256.remove_bit(63), true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 3, 3, 3],
                len: 65,
                bits: [0b1 | 1 << 24 | 1 << 26, 0, 0, 0],
            }
        );

        assert_eq!(bits256.remove_bit(0), true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 2, 2, 2],
                len: 64,
                bits: [1 << 23 | 1 << 25, 0, 0, 0],
            }
        );
    }

    #[test]
    fn test_bits256_set_bit() {
        let mut bits256 = Bits256 {
            ones: [0, 0, 0, 0],
            len: 256,
            bits: [0, 0, 0, 0],
        };

        bits256.set_bit(0, true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 1, 1, 1],
                len: 256,
                bits: [0b1, 0, 0, 0],
            }
        );

        bits256.set_bit(254, true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 1, 1, 1],
                len: 256,
                bits: [0b1, 0, 0, 1 << 62],
            }
        );

        bits256.set_bit(7, true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 2, 2, 2],
                len: 256,
                bits: [0b1000_0001, 0, 0, 1 << 62],
            }
        );

        bits256.set_bit(101, true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 2, 3, 3],
                len: 256,
                bits: [0b1000_0001, 1 << 37, 0, 1 << 62],
            }
        );

        bits256.set_bit(208, true);
        assert_eq!(
            bits256,
            Bits256 {
                ones: [0, 2, 3, 3],
                len: 256,
                bits: [0b1000_0001, 1 << 37, 0, 1 << 16 | 1 << 62]
            }
        );

        assert_eq!(
            (0..5).map(|b| bits256.select1(b)).collect::<Vec<_>>(),
            vec![0, 7, 101, 208, 254]
        );
    }

    #[test]
    fn test_bits256_select_rank_full_zeros() {
        let bits256 = Bits256 {
            ones: [0, 0, 0, 0],
            len: 256,
            bits: [0, 0, 0, 0],
        };

        assert_eq!(bits256.num_ones(), 0);
        assert_eq!(bits256.num_zeros(), 256);
        for i in 0..=255 {
            assert_eq!(bits256.rank1(i), 0);
            assert_eq!(bits256.rank0(i), i);
            assert_eq!(bits256.select0(i), i);
        }
    }

    #[test]
    fn test_bits256_select_rank_full_ones() {
        let bits256 = Bits256 {
            ones: [0, 64, 128, 3 * 64],
            len: 256,
            bits: [u64::max_value(); 4],
        };
        assert_eq!(bits256.num_ones(), 256);
        assert_eq!(bits256.num_zeros(), 0);
        for i in 0..=255 {
            assert_eq!(bits256.rank1(i), i);
            assert_eq!(bits256.rank0(i), 0);
            assert_eq!(bits256.select1(i), i);
        }
    }

    #[test]
    fn test_bits256_select_rank_half_ones() {
        let bits256 = Bits256 {
            ones: [0, 32, 64, 80],
            len: 160,
            bits: [
                0x5555_5555_5555_5555,
                0x5555_5555_5555_5555,
                0x0000_0000_5555_5555,
                0,
            ],
        };
        assert_eq!(bits256.num_ones(), 80);
        assert_eq!(bits256.num_zeros(), 80);

        for i in 0..160 {
            assert_eq!(bits256.rank0(i), i / 2);

            // For the bits ...01010101, rank1 should go
            //  0, 1, 1, 2, 2, 3, 3, 4, 4, ...
            // (we are exclusive, so bits256.rank1(0) == 0)
            assert_eq!(bits256.rank1(i), (i + 1) / 2);
        }

        for i in 0..80 {
            assert_eq!(bits256.select0(i), 2 * i + 1);
            assert_eq!(bits256.select1(i), 2 * i);
        }
    }

    proptest! {
        #[test]
        fn test_bits256_prop_insert(bits
            in proptest::collection::vec(proptest::bool::ANY, 1..255)) {
            let mut bits256 = Bits256 {
                ones: [0, 0, 0, 0],
                len: 0,
                bits: [0, 0, 0, 0],
            };

            for (i, bit) in bits.iter().cloned().enumerate() {
                bits256.insert_bit(i, bit);
            }

            // Test struct values
            assert_eq!(bits256.len as usize, bits.len());
            assert_eq!(
                bits256.ones,
                [
                    0,
                    bits.iter().zip(0..64).map(|(a, _b)| *a as u8).sum(),
                    bits.iter().zip(0..128).map(|(a, _b)| *a as u8).sum(),
                    bits.iter().zip(0..192).map(|(a, _b)| *a as u8).sum(),
                ]
            );

            assert_eq!(
                bits256.num_ones(),
                bits.iter().cloned().filter(|b| *b).count() as u32
            );
            assert_eq!(
                bits256.num_zeros(),
                bits.iter().cloned().filter(|b| !*b).count() as u32
            );

            for i in 0..bits256.num_ones() {
                assert!(bits[bits256.select1(i) as usize]);
            }
            for i in 0..bits256.num_zeros() {
                assert!(!bits[bits256.select0(i) as usize]);
            }

            let mut c0 = 0;
            let mut c1 = 0;
            for (i, bit) in bits.iter().cloned().enumerate() {
                assert_eq!(bits256.rank0(i as u32), c0);
                assert_eq!(bits256.rank1(i as u32), c1);

                c0 += !bit as u32;
                c1 += bit as u32;
            }
        }
    }
}