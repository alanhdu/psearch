pub(crate) fn binary_search_rank<T, F>(needle: T, len: usize, func: F) -> usize
where
    T: Ord,
    F: Fn(usize) -> T,
{
    let mut low = 0;
    let mut high = len;
    let mut mid = (low + high) / 2;

    while low < high {
        if needle < func(mid) {
            high = mid;
        } else if needle == func(mid) {
            break;
        } else {
            low = mid + 1;
        }
        mid = (low + high) / 2;
    }

    mid
}
