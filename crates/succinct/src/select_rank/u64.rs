use super::SelectRank;

/// parallel deposit
pub(crate) fn pdep(src: u64, mask: u64) -> u64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::_pdep_u64;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::_pdep_u64;

    unsafe { _pdep_u64(src, mask) }
}

/// parallel extract
pub(crate) fn pext(src: u64, mask: u64) -> u64 {
    #[cfg(target_arch = "x86")]
    use std::arch::x86::_pext_u64;
    #[cfg(target_arch = "x86_64")]
    use std::arch::x86_64::_pext_u64;

    unsafe { _pext_u64(src, mask) }
}

impl SelectRank for u64 {
    fn get_bit(&self, index: usize) -> bool {
        self & (1 << index) != 0
    }

    fn rank0(&self, index: usize) -> usize {
        index - self.rank1(index)
    }

    fn rank1(&self, index: usize) -> usize {
        if index == 0 {
            0
        } else {
            (self << (64 - index)).count_ones() as usize
        }
    }

    fn select0(&self, index: usize) -> usize {
        pdep(1 << index, !*self).trailing_zeros() as usize
    }

    fn select1(&self, index: usize) -> usize {
        pdep(1 << index, *self).trailing_zeros() as usize
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_u64_block() {
        let b: u64 = 0xFFFFFFFF_00000000;

        for i in 0..32 {
            assert_eq!(b.rank0(i), i);
            assert_eq!(b.rank1(i), 0);
        }
        for i in 32..64 {
            assert_eq!(b.rank0(i), 32);
            assert_eq!(b.rank1(i), i - 32);
        }

        for i in 0..32 {
            assert_eq!(b.select0(i), i);
            assert_eq!(b.select1(i), 32 + i);
        }
    }

    #[test]
    fn test_u64_ones() {
        let b = u64::max_value();
        for i in 0..64 {
            assert_eq!(b.rank0(i), 0);
            assert_eq!(b.rank1(i), i);

            assert_eq!(b.select1(i), i);
        }
    }
}
