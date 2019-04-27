#![allow(dead_code)]
use std::fmt;

#[allow(non_camel_case_types)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub(crate) struct u9x7(u64);

impl u9x7 {
    pub(crate) fn new(data: [u16; 7]) -> u9x7 {
        // data must fit into 9 bits
        debug_assert!(data.iter().all(|x| *x < 512));

        u9x7(
            u64::from(data[0])
                | (u64::from(data[1]) << 9)
                | (u64::from(data[2]) << 18)
                | (u64::from(data[3]) << 27)
                | (u64::from(data[4]) << 36)
                | (u64::from(data[5]) << 45)
                | (u64::from(data[6]) << 54),
        )
    }

    pub(crate) fn get(self, index: usize) -> u16 {
        debug_assert!(index < 8);
        ((self.0 >> (index * 9)) & 0b1_1111_1111) as u16
    }

    pub(crate) fn rank(self, needle: usize) -> usize {
        debug_assert!(needle <= 512);
        if needle >= 512 {
            return 7;
        }

        const SHIFT: u64 = 1
            | (1 << 9)
            | (1 << 18)
            | (1 << 27)
            | (1 << 36)
            | (1 << 45)
            | (1 << 54);
        const H: u64 = (1 << 8)
            | (1 << 17)
            | (1 << 26)
            | (1 << 35)
            | (1 << 44)
            | (1 << 53)
            | (1 << 62);

        // Algorithm from "Broadword Implementation of Rank/Select Queries":
        // x < y iff
        //     (( ((x|H)−(y&!H)) | x⊕y) ⊕ (x|!y) ) & H
        let x = self.0;
        let y = needle as u64 * SHIFT;
        let lt = ((((x | H) - (y & !H)) | (x ^ y)) ^ (x | !y)) & H;

        lt.count_ones() as usize
    }
}

impl fmt::Debug for u9x7 {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let values = [
            self.0 & 0b1_1111_1111,
            (self.0 >> 9) & 0b1_1111_1111,
            (self.0 >> 18) & 0b1_1111_1111,
            (self.0 >> 27) & 0b1_1111_1111,
            (self.0 >> 36) & 0b1_1111_1111,
            (self.0 >> 45) & 0b1_1111_1111,
            (self.0 >> 54) & 0b1_1111_1111,
        ];
        f.debug_tuple("u9x7").field(&values).finish()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_size() {
        assert_eq!(std::mem::size_of::<u9x7>(), std::mem::size_of::<u64>());
    }

    #[test]
    fn test_u9x7_rank() {
        let vec = u9x7::new([10, 40, 80, 255, 300, 300, 400]);

        for i in 0..=10 {
            assert_eq!(vec.rank(i), 0);
        }
        for i in 11..=40 {
            assert_eq!(vec.rank(i), 1);
        }
        for i in 41..=80 {
            assert_eq!(vec.rank(i), 2);
        }
        for i in 81..=255 {
            assert_eq!(vec.rank(i), 3);
        }
        for i in 256..=300 {
            assert_eq!(vec.rank(i), 4);
        }
        for i in 301..=400 {
            assert_eq!(vec.rank(i), 6);
        }
        for i in 401..=512 {
            assert_eq!(vec.rank(i), 7);
        }
    }
}
