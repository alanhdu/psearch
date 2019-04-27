#![allow(dead_code)]
#[allow(non_camel_case_types)]
pub(crate) struct u8x3(u32);

impl u8x3 {
    pub(crate) fn new(data: [u8; 3]) -> u8x3 {
        let a = data[0] as u32;
        let b = data[1] as u32;
        let c = data[2] as u32;

        // Layout is:
        // 0000 cccc cccc 00bb bbbb bb00 aaaa aaaa
        u8x3(a | (b << 10) | (c << 20))
    }

    pub(crate) fn rank(&self, needle: usize) -> usize {
        debug_assert!(needle <= 256);

        const SHIFT: u32 = 1 | (1 << 10) | (1 << 20);
        const MASK: u32 = (1 << 9) | (1 << 19) | (1 << 29);

        // Derivation of algorithm, where x and y are k-bit numbers:
        //      x   < y
        //      0   <= y - x - 1
        //      2^k <= y + (2^k - x - 1)
        //      2^k <= y + (x ^ 01111...)
        // Now, 2^k = 10000..., so this can be done with a single
        // bitwise & and we get:
        //      x < y = (y + (x ^ 01111...)) & 10000...
        //
        // Algorithm taken from "BitWeaving: Fast Scans for Main Memory
        // Data Processing"
        let needle = (needle as u32) * SHIFT;
        let result = (needle + (self.0 ^ !MASK)) & MASK;

        // Extract number of 1s (which is equal to the rank)
        // TODO: benchmark vs popcnt
        (result.overflowing_mul(SHIFT).0 as usize >> 29)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<u8x3>(), std::mem::size_of::<u32>());
    }

    #[test]
    fn test_u8x3_rank() {
        let vec = u8x3::new([10, 40, 80]);

        for i in 0..=10 {
            assert_eq!(vec.rank(i), 0);
        }
        for i in 11..=40 {
            assert_eq!(vec.rank(i), 1);
        }
        for i in 41..=80 {
            assert_eq!(vec.rank(i), 2);
        }
        for i in 81..=256 {
            assert_eq!(vec.rank(i), 3);
        }
    }
}
