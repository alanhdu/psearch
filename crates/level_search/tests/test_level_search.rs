use std::collections::BTreeSet;

use proptest::prelude::*;
use level_search::{xfast::XFastSet, yfast::YFastSet};

proptest! {
    #[test]
    #[ignore]
    fn proptest_xfast_insertions_u32(
        items in prop::collection::vec(any::<u32>(), 1..10_000)
    ) {
        let mut xfast = XFastSet::new();
        let mut expected = BTreeSet::new();
        for item in items.iter() {
            xfast.insert(*item);
            expected.insert(*item);

            assert_eq!(xfast.len(), expected.len());
        }

        let items = expected.iter().cloned().collect::<Vec<_>>();
        assert_eq!(xfast.iter().collect::<Vec<_>>(), items);

        for (i, item) in items.iter().cloned().enumerate() {
            assert!(xfast.contains(item));
            assert_eq!(xfast.predecessor(item), Some(item));
            assert_eq!(xfast.successor(item), Some(item));

            if item > 0 && !expected.contains(&(item - 1)) {
                assert_eq!(xfast.successor(item - 1), Some(item));

                assert_eq!(
                    xfast.predecessor(item - 1),
                    if i > 0 { Some(items[i - 1]) } else { None }
                );
            }

            if item < u32::max_value() && !expected.contains(&(item + 1)) {
                assert_eq!(xfast.predecessor(item + 1), Some(item));

                assert_eq!(
                    xfast.successor(item + 1),
                    if i < items.len() - 1 { Some(items[i + 1]) } else { None }
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn proptest_xfast_insertions_u64(
        items in prop::collection::vec(any::<u64>(), 1..10_000)
    ) {
        let mut xfast = XFastSet::new();
        let mut expected = BTreeSet::new();
        for item in items.iter() {
            xfast.insert(*item);
            expected.insert(*item);

            assert_eq!(xfast.len(), expected.len());
        }

        let items = expected.iter().cloned().collect::<Vec<_>>();
        assert_eq!(xfast.iter().collect::<Vec<_>>(), items);

        for (i, item) in items.iter().cloned().enumerate() {
            assert!(xfast.contains(item));
            assert_eq!(xfast.predecessor(item), Some(item));
            assert_eq!(xfast.successor(item), Some(item));

            if item > 0 && !expected.contains(&(item - 1)) {
                assert_eq!(xfast.successor(item - 1), Some(item));

                assert_eq!(
                    xfast.predecessor(item - 1),
                    if i > 0 { Some(items[i - 1]) } else { None }
                );
            }

            if item < u64::max_value() && !expected.contains(&(item + 1)) {
                assert_eq!(xfast.predecessor(item + 1), Some(item));

                assert_eq!(
                    xfast.successor(item + 1),
                    if i < items.len() - 1 { Some(items[i + 1]) } else { None }
                );
            }
        }
    }

    #[test]
    #[ignore]
    fn proptest_yfast_insertions(
        items in prop::collection::vec(any::<u32>(), 1..10_000)
    ) {
        let mut yfast = YFastSet::new();
        let mut expected = BTreeSet::new();
        for item in items.iter() {
            yfast.insert(*item);
            expected.insert(*item);

            assert_eq!(yfast.len(), expected.len());
        }

        let items = expected.iter().cloned().collect::<Vec<_>>();
        assert_eq!(yfast.iter().collect::<Vec<_>>(), items);

        for (i, item) in items.iter().cloned().enumerate() {
            assert!(yfast.contains(item));
            assert_eq!(yfast.predecessor(item), Some(item));
            assert_eq!(yfast.successor(item), Some(item));

            if item > 0 && !expected.contains(&(item - 1)) {
                assert_eq!(yfast.successor(item - 1), Some(item));

                assert_eq!(
                    yfast.predecessor(item - 1),
                    if i > 0 { Some(items[i - 1]) } else { None }
                );
            }

            if item < u32::max_value() && !expected.contains(&(item + 1)) {
                assert_eq!(yfast.predecessor(item + 1), Some(item));

                assert_eq!(
                    yfast.successor(item + 1),
                    if i < items.len() - 1 { Some(items[i + 1]) } else { None }
                );
            }
        }
    }
}
