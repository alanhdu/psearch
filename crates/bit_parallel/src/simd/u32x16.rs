#[cfg(target_arch = "x86")]
use std::arch::x86::*;
#[cfg(target_arch = "x86_64")]
use std::arch::x86_64::*;

const INCREMENT: [u32; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 1, 1, 1, 1, 1, 1, 1, 1];

#[allow(clippy::cast_ptr_alignment)]
unsafe fn loadu(src: &[u32]) -> __m256i {
    debug_assert!(src.iter().all(|v| *v < i32::max_value() as u32));

    // loadu allows arbitrary alignment
    #[allow(clippy::cast_ptr_alignment)]
    _mm256_loadu_si256(src as *const _ as *const _)
}

#[allow(clippy::cast_ptr_alignment)]
unsafe fn storeu(dest: &mut [u32], src: __m256i) {
    // loadu allows arbitrary alignment
    _mm256_storeu_si256(dest as *mut _ as *mut _, src);
}

pub fn rank(haystack: &[u32; 16], needle: u32) -> u8 {
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

pub fn rank_diff(big: &[u32; 16], small: &[u32; 16], needle: u32) -> u8 {
    unsafe {
        let needle = _mm256_set1_epi32(std::mem::transmute::<u32, i32>(needle));

        let haystack1 = _mm256_sub_epi32(loadu(big), loadu(small));
        let cmp1 =
            _mm256_cmpeq_epi32(_mm256_min_epu32(haystack1, needle), needle);

        let haystack2 = _mm256_sub_epi32(loadu(&big[8..]), loadu(&small[8..]));
        let cmp2 =
            _mm256_cmpeq_epi32(_mm256_min_epu32(haystack2, needle), needle);

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

pub fn increment(values: &mut [u32; 16], mut pos: usize) {
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

pub fn decrement(values: &mut [u32; 16], mut pos: usize) {
    unsafe {
        if pos < 8 {
            let half = loadu(values);
            let inc = loadu(&INCREMENT[8 - pos..]);

            storeu(values, _mm256_sub_epi32(half, inc));
            pos = 8;
        }
        let half = loadu(&values[8..]);
        let inc = loadu(&INCREMENT[16 - pos..]);
        storeu(&mut values[8..], _mm256_sub_epi32(half, inc));
    }
}

pub fn split(src: &mut [u32; 16], dest: &mut [u32; 16]) {
    unsafe {
        let bottom = _mm256_set1_epi32(std::mem::transmute::<u32, i32>(src[7]));
        let top = _mm256_set1_epi32(std::mem::transmute::<u32, i32>(
            src[15] - src[7],
        ));

        storeu(dest, _mm256_sub_epi32(loadu(&src[8..]), bottom));
        storeu(&mut src[8..], bottom);
        storeu(&mut dest[8..], top);
    }
}
