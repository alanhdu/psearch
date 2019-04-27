#![allow(dead_code)]

#[allow(non_camel_case_types)]
pub(crate) struct u9x7(u64);

impl u9x7 {
    pub(crate) fn new(data: [u16; 7]) -> u9x7 {
        // data must fit into 9 bits
        debug_assert!(data.iter().all(|x| *x < 512));

        let a = data[0] as u64;
        let b = data[1] as u64;
        let c = data[2] as u64;
        let d = data[3] as u64;
        let e = data[4] as u64;
        let f = data[5] as u64;
        let g = data[6] as u64;

        u9x7(
            a | (b << 9)
                | (c << 18)
                | (d << 27)
                | (e << 36)
                | (f << 45)
                | (g << 54),
        )
    }

    pub(crate) fn rank(&self, needle: usize) -> usize {
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
