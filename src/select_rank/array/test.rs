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
