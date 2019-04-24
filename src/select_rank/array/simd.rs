#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

const INCREMENT: [u32; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1];

#[allow(clippy::cast_ptr_alignment)]
unsafe fn loadu(haystack: &[u32]) -> __m256i {
    // loadu allows arbitrary alignment
    #[allow(clippy::cast_ptr_alignment)]
    _mm256_loadu_si256(haystack as *const _ as *const _)
}

#[allow(clippy::cast_ptr_alignment)]
unsafe fn storeu(dest: &mut [u32], src: __m256i) {
    // loadu allows arbitrary alignment
    _mm256_storeu_si256(dest as *mut _ as *mut _, src);
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

pub(crate) fn increment(values: &mut [u32; 16], mut pos: usize) {
    debug_assert!(values.iter().all(|v| *v < i32::max_value() as u32));
    unsafe {
        if pos < 8 {
            let half = loadu(values);
            let inc = loadu(&INCREMENT[8 - pos..]);

            storeu(values, _mm256_add_epi32(half, inc));
            pos = 8;
        }
        let half = loadu(&values[8..]);
        let inc = loadu(&INCREMENT[16 - pos..]);
        storeu(&mut values[8..], _mm256_add_epi32(half, inc));
    }
}