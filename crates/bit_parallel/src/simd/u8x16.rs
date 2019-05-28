#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

#[allow(clippy::cast_ptr_alignment)]
unsafe fn loadu(src: &[u8]) -> __m128i {
    // loadu allows arbitrary alignment
    #[allow(clippy::cast_ptr_alignment)]
    _mm_loadu_si128(src as *const _ as *const _)
}

pub fn rank(haystack: &[u8; 16], needle: u8) -> usize {
    unsafe {
        let needle = _mm_set1_epi8(std::mem::transmute::<u8, i8>(needle));
        let cmp = _mm_cmpeq_epi8(
            _mm_min_epu8(loadu(haystack), needle),
            needle,
        );
        let mask = std::mem::transmute::<i32, u32>(_mm_movemask_epi8(cmp));
        if mask == 0 {
            16
        } else {
            (mask.trailing_zeros() / 2) as usize
        }
    }
}
