use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};

fn simd_rank(keys: [u8; 16], key: u8, nchildren: usize) -> usize {
    use std::arch::x86_64::*;
    let matches = unsafe {
        let haystack = _mm_loadu_si128(&keys as *const _ as *const __m128i);
        assert_eq!(std::mem::transmute::<__m128i, [u8; 16]>(haystack), keys);
        let needle = _mm_set1_epi8(std::mem::transmute::<u8, i8>(key));
        let cmp = _mm_cmpeq_epi8(_mm_min_epu8(haystack, needle), needle);
        _mm_movemask_epi8(cmp)
    };
    std::cmp::min(matches.trailing_zeros() as usize, nchildren)
}

fn binary_checked_rank(keys: [u8; 16], key: u8, nchildren: usize) -> usize {
    let mut left = 0;
    let mut right = nchildren;

    let mut mid = (left + right) / 2;
    while left < right {
        if keys[mid] < key {
            left = mid + 1;
        } else if key == keys[mid] {
            break;
        } else {
            // Not mid-1, because `right` might be the successor of key
            right = mid;
        }
        mid = (left + right) / 2;
    }
    mid
}

fn binary_unchecked_rank(keys: [u8; 16], key: u8, nchildren: usize) -> usize {
    let mut left = 0;
    let mut right = nchildren;

    let mut mid = (left + right) / 2;
    while left < right {
        let m = unsafe { *keys.get_unchecked(mid) };
        if m < key {
            left = mid + 1;
        } else if key == m {
            break;
        } else {
            // Not mid-1, because `right` might be the successor of key
            right = mid;
        }
        mid = (left + right) / 2;
    }
    mid
}

fn linear_rank(keys: [u8; 16], key: u8, nchildren: usize) -> usize {
    for (i, k) in keys[..nchildren].iter().cloned().enumerate() {
        if key <= k {
            return i;
        }
    }
    return nchildren;
}

fn unrolled_rank(keys: [u8; 16], key: u8, nchildren: usize) -> usize {
    let max = nchildren / 4;
    for i in 0..max {
        if keys[i * 4 + 3] <= key {
            for j in 0..3 {
                if keys[i * 4 + j] <= key {
                    return j;
                }
            }
            return i;
        }
    }

    for j in (nchildren - 4 * max)..nchildren {
        if keys[j] <= key {
            return j;
        }
    }
    return nchildren;
}

const EXPAND: [[u8; 16]; 15] = [
    [0x80, 0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 0x80, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 0x80, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 0x80, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 0x80, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 0x80, 5, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 0x80, 6, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 0x80, 7, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 0x80, 8, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 0x80, 9, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 0x80, 10, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 0x80, 11, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 0x80, 12, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 0x80, 13, 15],
    [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 0x80, 15],
];

pub fn simd_expand(keys: &mut [u8; 16], pos: usize) {
    use std::arch::x86_64::*;
    unsafe {
        let haystack = _mm_loadu_si128(keys as *const _ as *const _);
        let expand =
            _mm_loadu_si128(&EXPAND[pos] as *const _ as *const __m128i);
        let output = _mm_shuffle_epi8(haystack, expand);
        _mm_storeu_si128(keys as *const _ as *mut _, output);
    }
}

pub fn copy_expand(keys: &mut [u8; 16], pos: usize) {
    unsafe {
        // 14 to avoid overwriting the last pointer
        std::ptr::copy(&keys[pos], &mut keys[pos + 1], 14 - pos);
        keys[pos] = 0;
    }
}

pub fn loop_expand(keys: &mut [u8; 16], pos: usize) {
    for i in (1 + pos..15).rev() {
        keys[i - 1] = keys[i];
    }
    keys[pos] = 0;
}

fn criterion_benchmark(c: &mut Criterion) {
    let keys16: [u8; 16] =
        [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 24, 26, 28, 30];
    for (key, expected) in [(1, 1), (16, 8), (17, 9), (31, 16)].iter().cloned()
    {
        assert_eq!(linear_rank(keys16, key, 16), expected);
        assert_eq!(simd_rank(keys16, key, 16), expected);
        assert_eq!(binary_checked_rank(keys16, key, 16), expected);
        assert_eq!(binary_unchecked_rank(keys16, key, 16), expected);
    }

    let keys12: [u8; 16] =
        [0, 2, 4, 6, 8, 10, 12, 14, 16, 18, 20, 22, 0, 0, 0, 12];
    for (key, expected) in [(1, 1), (16, 8), (17, 9), (31, 12)].iter().cloned()
    {
        assert_eq!(linear_rank(keys12, key, 12), expected);
        assert_eq!(simd_rank(keys12, key, 12), expected);
        assert_eq!(binary_checked_rank(keys12, key, 12), expected);
        assert_eq!(binary_unchecked_rank(keys12, key, 12), expected);
    }

    let keys4: [u8; 16] = [0, 2, 4, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4];
    for (key, expected) in [(1, 1), (16, 4), (17, 4), (31, 4)].iter().cloned() {
        assert_eq!(linear_rank(keys12, key, 4), expected);
        assert_eq!(simd_rank(keys12, key, 4), expected);
        assert_eq!(binary_checked_rank(keys12, key, 4), expected);
        assert_eq!(binary_unchecked_rank(keys12, key, 4), expected);
    }

    c.bench(
        "rank_16",
        ParameterizedBenchmark::new(
            "simd",
            move |b, i| b.iter(|| black_box(simd_rank(keys16, *i, 16))),
            vec![1, 16, 17, 31],
        )
        .with_function("binary_checked", move |b, i| {
            b.iter(|| black_box(binary_checked_rank(keys16, *i, 16)))
        })
        .with_function("binary_unchecked", move |b, i| {
            b.iter(|| black_box(binary_unchecked_rank(keys16, *i, 16)))
        })
        .with_function("unrolled_rank", move |b, i| {
            b.iter(|| black_box(unrolled_rank(keys16, *i, 16)))
        })
        .with_function("linear", move |b, i| {
            b.iter(|| black_box(linear_rank(keys16, *i, 16)))
        }),
    );

    c.bench(
        "rank_12",
        ParameterizedBenchmark::new(
            "simd",
            move |b, i| b.iter(|| black_box(simd_rank(keys12, *i, 12))),
            vec![1, 12, 17, 31],
        )
        .with_function("binary_checked", move |b, i| {
            b.iter(|| black_box(binary_checked_rank(keys12, *i, 12)))
        })
        .with_function("binary_unchecked", move |b, i| {
            b.iter(|| black_box(binary_unchecked_rank(keys12, *i, 12)))
        })
        .with_function("unrolled_rank", move |b, i| {
            b.iter(|| black_box(unrolled_rank(keys12, *i, 12)))
        })
        .with_function("linear", move |b, i| {
            b.iter(|| black_box(linear_rank(keys12, *i, 12)))
        }),
    );

    c.bench(
        "rank_4",
        ParameterizedBenchmark::new(
            "simd",
            move |b, i| b.iter(|| black_box(simd_rank(keys4, *i, 4))),
            vec![1, 4, 17, 31],
        )
        .with_function("binary_checked", move |b, i| {
            b.iter(|| black_box(binary_checked_rank(keys4, *i, 4)))
        })
        .with_function("binary_unchecked", move |b, i| {
            b.iter(|| black_box(binary_unchecked_rank(keys4, *i, 4)))
        })
        .with_function("unrolled_rank", move |b, i| {
            b.iter(|| black_box(unrolled_rank(keys4, *i, 4)))
        })
        .with_function("linear", move |b, i| {
            b.iter(|| black_box(linear_rank(keys4, *i, 4)))
        }),
    );

    c.bench(
        "expand",
        ParameterizedBenchmark::new(
            "simd",
            move |b, i| {
                b.iter(|| {
                    let mut keys: [u8; 16] =
                        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0, 16];
                    simd_expand(&mut keys, *i);
                    black_box(keys)
                })
            },
            vec![0, 7, 14],
        )
        .with_function("copy", |b, i| {
            b.iter(|| {
                let mut keys: [u8; 16] =
                    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0, 16];
                copy_expand(&mut keys, *i);
                black_box(keys)
            })
        })
        .with_function("loop", |b, i| {
            b.iter(|| {
                let mut keys: [u8; 16] =
                    [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 0, 16];
                loop_expand(&mut keys, *i);
                black_box(keys)
            })
        }),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
