#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[allow(clippy::cast_ptr_alignment)]
unsafe fn loadu(src: &[u16]) -> __m256i {
    // loadu allows arbitrary alignment
    #[allow(clippy::cast_ptr_alignment)]
    _mm256_loadu_si256(src as *const _ as *const _)
}

pub(crate) fn rank(haystack: &[u16; 32], needle: u16) -> usize {
    unsafe {
        let needle = _mm256_set1_epi16(std::mem::transmute::<u16, i16>(needle));
        let cmp1 = _mm256_cmpeq_epi16(
            _mm256_min_epu16(loadu(haystack), needle),
            needle,
        );
        let cmp2 = _mm256_cmpeq_epi16(
            _mm256_min_epu16(loadu(&haystack[16..]), needle),
            needle,
        );

        let mask1 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp1));
        let mask2 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp2));
        let mask = (u64::from(mask2) << 32) | u64::from(mask1);
        if mask == 0 {
            32
        } else {
            (mask.trailing_zeros() / 2) as usize
        }
    }
}

pub(crate) fn rank_diff(
    big: &[u16; 32],
    small: &[u16; 32],
    needle: u16,
) -> usize {
    unsafe {
        let needle = _mm256_set1_epi16(std::mem::transmute::<u16, i16>(needle));
        let haystack1 = _mm256_sub_epi16(loadu(big), loadu(small));
        let cmp1 =
            _mm256_cmpeq_epi16(_mm256_min_epu16(haystack1, needle), needle);

        let haystack2 = _mm256_sub_epi16(loadu(&big[16..]), loadu(&small[16..]));
        let cmp2 =
            _mm256_cmpeq_epi16(_mm256_min_epu16(haystack2, needle), needle);

        let mask1 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp1));
        let mask2 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp2));
        let mask = (u64::from(mask2) << 32) | u64::from(mask1);

        if mask == 0 {
            32
        } else {
            (mask.trailing_zeros() / 2) as usize
        }
    }
}
