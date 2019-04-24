#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

unsafe fn loadu(haystack: &[u32]) -> __m256i {
    // loadu allows arbitrary alignment
    #[allow(clippy::cast_ptr_alignment)]
    _mm256_loadu_si256(haystack as *const _ as *const _)
}

pub(crate) fn index(haystack: &[u32; 16], needle: u32) -> Option<u8> {
    unsafe {
        let needle = _mm256_set1_epi32(std::mem::transmute::<u32, i32>(needle));
        let cmp1 = _mm256_cmpeq_epi32(loadu(haystack), needle);
        let cmp2 = _mm256_cmpeq_epi32(loadu(&haystack[8..]), needle);

        let mask1 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp1));
        let mask2 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp2));

        let mask = (u64::from(mask2) << 32) | u64::from(mask1);

        if mask == 0 {
            None
        } else {
            Some((mask.trailing_zeros() / 4) as u8)
        }
    }
}

pub(crate) fn rank(haystack: &[u32; 16], needle: u32) -> u8 {
    unsafe {
        let needle = _mm256_set1_epi32(std::mem::transmute::<u32, i32>(needle));
        let cmp1 = _mm256_cmpeq_epi32(
            _mm256_min_epu32(loadu(haystack), needle),
            needle,
        );
        let cmp2 = _mm256_cmpeq_epi32(
            _mm256_min_epu32(loadu(&haystack[8..]), needle),
            needle,
        );

        let mask1 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp1));
        let mask2 = std::mem::transmute::<i32, u32>(_mm256_movemask_epi8(cmp2));

        let mask = (u64::from(mask2) << 32) | u64::from(mask1);
        if mask == 0 {
            16
        } else {
            (mask.trailing_zeros() / 4) as u8
        }
    }
}
