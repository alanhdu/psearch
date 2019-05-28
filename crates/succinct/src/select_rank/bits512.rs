use super::{u64::pdep, SelectRank};
use bit_parallel::u9x7::u9x7;

const INCREMENT: [u64; 8] = [
    1 | (1 << 9) | (1 << 18) | (1 << 27) | (1 << 36) | (1 << 45) | (1 << 54),
    (1 << 9) | (1 << 18) | (1 << 27) | (1 << 36) | (1 << 45) | (1 << 54),
    (1 << 18) | (1 << 27) | (1 << 36) | (1 << 45) | (1 << 54),
    (1 << 27) | (1 << 36) | (1 << 45) | (1 << 54),
    (1 << 36) | (1 << 45) | (1 << 54),
    (1 << 45) | (1 << 54),
    (1 << 54),
    0,
];

#[derive(Debug, Eq, PartialEq)]
pub struct Bits512 {
    pub(super) n_ones: u9x7,
    pub(super) len: usize,
    pub(super) bits: [u64; 8],
}

impl Bits512 {
    pub fn new() -> Bits512 {
        Bits512 {
            bits: [0; 8],
            len: 0,
            n_ones: u9x7(0),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn len(&self) -> usize {
        self.len as usize
    }

    pub fn is_full(&self) -> bool {
        debug_assert!(self.len <= 512);
        self.len == 512
    }

    pub fn num_ones(&self) -> u32 {
        self.n_ones.get(6) as u32 + self.bits[7].count_ones()
    }

    pub fn num_zeros(&self) -> u32 {
        self.len as u32 - self.num_ones()
    }

    /// Insert a bit at our index
    pub fn insert(&mut self, index: usize, bit: bool) {
        debug_assert!(!self.is_full());
        debug_assert!(index <= self.len as usize);

        let upper = index / 64;
        let lower = index % 64;

        let mut last = self.bits[upper] >> 63;
        self.n_ones.0 += (bit as u64) * INCREMENT[upper];
        self.n_ones.0 -= last * INCREMENT[upper];

        // TODO: SIMD accelerate (can shift 256 bits at a time)
        self.bits[upper] = pdep(self.bits[upper], !(1 << lower))
            | ((bit as u64) * (1 << lower));

        for upper in (upper + 1)..=(self.len / 64) {
            let old = last;
            last = self.bits[upper] >> 63;

            self.n_ones.0 += old * INCREMENT[upper];
            self.n_ones.0 -= last * INCREMENT[upper];

            self.bits[upper] = (self.bits[upper] << 1) | (old as u64);
        }

        self.len += 1;
    }

    pub fn set_bit(&mut self, index: usize, bit: bool) {
        debug_assert!(index < self.len as usize);
        let upper = index / 64;
        let lower = index % 64;

        let prev_bit = self.bits[upper] & (1 << lower) != 0;
        if prev_bit != bit {
            if bit {
                self.bits[upper] |= 1 << lower;
                self.n_ones.0 += INCREMENT[upper];
            } else {
                self.bits[upper] &= !(1 << lower);
                self.n_ones.0 -= INCREMENT[upper];
            }
        }
    }

    pub fn split(&mut self) -> Bits512 {
        debug_assert!(self.is_full());

        let mid = self.n_ones.get(3) as u64;

        let value = (self.n_ones.0 >> (4 * 9))
            + (self.num_ones() as u64)
                * ((1 << 27) | (1 << 36) | (1 << 45) | (1 << 54))
            - mid
                * (1 | (1 << 9)
                    | (1 << 18)
                    | (1 << 27)
                    | (1 << 36)
                    | (1 << 45)
                    | (1 << 54));

        let new = Bits512 {
            bits: [
                self.bits[4],
                self.bits[5],
                self.bits[6],
                self.bits[7],
                0,
                0,
                0,
                0,
            ],
            len: 256,
            n_ones: u9x7(value),
        };

        self.len = 256;
        self.bits[4] = 0;
        self.bits[5] = 0;
        self.bits[6] = 0;
        self.bits[7] = 0;

        let clear_upper = self.n_ones.0 & ((1 << 36) - 1);
        self.n_ones =
            u9x7(clear_upper + mid * ((1 << 36) | (1 << 45) | (1 << 54)));

        new
    }

    pub fn iter(&self) -> impl Iterator<Item = bool> + '_ {
        (0..self.len()).map(move |i| {
            let upper = i / 64;
            let lower = i % 64;
            self.bits[upper] & (1 << lower) != 0
        })
    }
}

impl From<bool> for Bits512 {
    fn from(bit: bool) -> Bits512 {
        Bits512 {
            n_ones: u9x7::new([bit as u16; 7]),
            bits: [bit as u64, 0, 0, 0, 0, 0, 0, 0],
            len: 1,
        }
    }
}

impl SelectRank for Bits512 {
    fn get_bit(&self, index: usize) -> bool {
        assert!(index < 512);
        let upper = index / 64;
        let lower = index % 64;

        self.bits[upper] & (1 << lower) != 0
    }

    /// Return the number of 0s before the `i`th position
    fn rank0(&self, index: usize) -> usize {
        index - self.rank1(index)
    }

    /// Return the number of 1s before the `i`th position
    fn rank1(&self, index: usize) -> usize {
        let upper = index / 64;
        let lower = index % 64;

        let bits = if lower == 0 {
            0
        } else {
            self.bits[upper].rank1(lower)
        };

        let bytes = if upper == 0 {
            0
        } else {
            self.n_ones.get(upper - 1) as usize
        };

        bytes + bits
    }

    /// Return the position of the `i`th 0 (0-indexed)
    fn select0(&self, mut index: usize) -> usize {
        assert!(index < self.len as usize);
        let rank = self.n_ones.rank_zero(index + 1);
        index -= if rank == 0 {
            0
        } else {
            64 * rank - self.n_ones.get(rank - 1) as usize
        };

        rank * 64 + self.bits[rank].select0(index)
    }

    /// Return the position of the `i`th 1 (0-indexed)
    fn select1(&self, mut index: usize) -> usize {
        assert!(index < self.len as usize);

        let rank = self.n_ones.rank(index + 1);
        index -= if rank == 0 {
            0
        } else {
            self.n_ones.get(rank - 1) as usize
        };
        64 * rank + self.bits[rank].select1(index)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_sizes() {
        assert_eq!(
            std::mem::size_of::<Bits512>(),
            10 * std::mem::size_of::<u64>()
        );
    }

    #[test]
    fn test_bits512_insert() {
        let mut bits = Bits512::new();
        bits.len = 100;

        bits.insert(0, true);
        bits.insert(25, true);
        bits.insert(25, false);
        bits.insert(25, true);
        bits.insert(64, true);

        assert_eq!(
            bits,
            Bits512 {
                n_ones: u9x7::new([3, 4, 4, 4, 4, 4, 4]),
                len: 105,
                bits: [0b1 | 1 << 25 | 1 << 27, 1, 0, 0, 0, 0, 0, 0],
            }
        );
    }

    #[test]
    fn test_bits512_split() {
        let mut first = Bits512 {
            n_ones: u9x7::new([64, 128, 192, 256, 5 * 64, 6 * 64, 7 * 64]),
            len: 512,
            bits: [u64::max_value(); 8],
        };
        let second = first.split();

        let expected = Bits512 {
            n_ones: u9x7::new([64, 128, 192, 256, 256, 256, 256]),
            len: 256,
            bits: [
                u64::max_value(),
                u64::max_value(),
                u64::max_value(),
                u64::max_value(),
                0,
                0,
                0,
                0,
            ],
        };

        assert_eq!(second, expected);
        assert_eq!(first, expected);
    }

    #[test]
    fn test_bits512_select_rank_full_zeros() {
        let bits = Bits512 {
            n_ones: u9x7(0),
            len: 512,
            bits: [0; 8],
        };

        assert_eq!(bits.num_ones(), 0);
        assert_eq!(bits.num_zeros(), 512);
        for i in 0..512 {
            assert_eq!(bits.rank1(i), 0);
            assert_eq!(bits.rank0(i), i);
            assert_eq!(bits.select0(i), i);
        }
    }

    #[test]
    fn test_bits512_select_rank_full_ones() {
        let bits = Bits512 {
            n_ones: u9x7::new([64, 128, 192, 256, 320, 384, 448]),
            len: 512,
            bits: [u64::max_value(); 8],
        };

        assert_eq!(bits.num_ones(), 512);
        assert_eq!(bits.num_zeros(), 0);
        for i in 0..512 {
            assert_eq!(bits.rank1(i), i);
            assert_eq!(bits.rank0(i), 0);
            assert_eq!(bits.select1(i), i);
        }
    }

    use proptest::prelude::*;
    proptest! {
        #[test]
        fn test_bits512_prop_insert(input
            in prop::collection::vec(any::<(u8, bool)>(), 1..512)
        ) {
            let mut bits512 = Bits512::new();
            let mut bits = Vec::with_capacity(input.len());

            for (order, bit) in input.iter().cloned() {
                let order = order as usize % (bits.len() + 1);
                bits512.insert(order, bit);
                bits.insert(order, bit);

                prop_assert_eq!(&bits512.iter().collect::<Vec<_>>(), &bits);

                for j in 1..7 {
                    prop_assert!(bits512.n_ones.get(j) >= bits512.n_ones.get(j - 1));
                }
            }

            // Test struct values
            prop_assert_eq!(bits512.len as usize, bits.len());
            prop_assert_eq!(
                bits512.num_ones(),
                bits.iter().cloned().filter(|b| *b).count() as u32
            );
            prop_assert_eq!(
                bits512.num_zeros(),
                bits.iter().cloned().filter(|b| !*b).count() as u32
            );

            for i in 0..bits512.num_ones() {
                prop_assert!(bits[bits512.select1(i as usize)]);
            }
            for i in 0..bits512.num_zeros() {
                prop_assert!(!bits[bits512.select0(i as usize)]);
            }

            let mut c0 = 0;
            let mut c1 = 0;
            for (i, bit) in bits.iter().cloned().enumerate() {
                prop_assert_eq!(bits512.rank0(i), c0);
                prop_assert_eq!(bits512.rank1(i), c1);

                c0 += !bit as usize;
                c1 += bit as usize;
            }
        }
    }
}
