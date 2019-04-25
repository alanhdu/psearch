use super::*;

#[test]
fn test_index() {
    let haystack: [u32; 16] = [
        0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987,
    ];

    for (i, h) in haystack.iter().cloned().enumerate() {
        assert_eq!(index(&haystack, h), Some(i as u8));
    }
    assert_eq!(index(&haystack, 123), None);
}

#[test]
fn test_rank() {
    let haystack: [u32; 16] = [
        0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987,
    ];

    for (i, h) in haystack.iter().cloned().enumerate() {
        assert_eq!(rank(&haystack, h), i as u8);
    }

    assert_eq!(rank(&haystack, 986), 15);
    assert_eq!(rank(&haystack, 988), 16);
}

#[test]
fn test_rank_diff() {
    let big: [u32; 16] = [
        0, 2, 4, 6, 10, 16, 26, 42, 68, 110, 178, 288, 466, 754, 1220, 1974,
    ];

    let small: [u32; 16] = [
        0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987,
    ];

    for (i, h) in small.iter().cloned().enumerate() {
        assert_eq!(rank_diff(&big, &small, h), i as u8);
    }
    assert_eq!(rank_diff(&big, &small, 986), 15);
    assert_eq!(rank_diff(&big, &small, 988), 16);
}

#[test]
fn test_increment() {
    let mut values: [u32; 16] = [0; 16];

    let expected = [
        [1; 16],
        [1, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2],
        [1, 2, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3],
        [1, 2, 3, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4],
        [1, 2, 3, 4, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5, 5],
        [1, 2, 3, 4, 5, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6, 6],
        [1, 2, 3, 4, 5, 6, 7, 7, 7, 7, 7, 7, 7, 7, 7, 7],
        [1, 2, 3, 4, 5, 6, 7, 8, 8, 8, 8, 8, 8, 8, 8, 8],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 9, 9, 9, 9, 9, 9, 9],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 10, 10, 10, 10, 10, 10],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 11, 11, 11, 11, 11],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 12, 12, 12, 12],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 13, 13, 13],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 14, 14],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 15],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
        [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16],
    ];

    for i in 0..=16 {
        increment(&mut values, i);
        assert_eq!(values, expected[i as usize]);
    }
}

#[test]
fn test_split() {
    let mut src: [u32; 16] = [
        0, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987,
    ];
    let mut dest: [u32; 16] = [0; 16];

    split(&mut src, &mut dest);

    assert_eq!(
        src,
        //  2  3  4  5  6  7   8 ...
        [0, 1, 2, 3, 5, 8, 13, 21, 21, 21, 21, 21, 21, 21, 21, 21]
    );
    assert_eq!(
        dest,
        [
            13, 34, 68, 123, 212, 356, 589, 966, 966, 966, 966, 966, 966, 966,
            966, 966
        ]
    );
}
