use std::collections::{BTreeSet, HashSet};
use std::iter::FromIterator;

use proptest::prelude::*;
use psearch::xfast::XFastSet;

proptest! {
    #[test]
    fn xfast_iteration(items: BTreeSet<u32>) {
        let mut xfast = XFastSet::new();
        for item in items.iter() {
            xfast.insert(*item);
        }

        assert_eq!(
            xfast.iter().collect::<Vec<_>>(),
            items.iter().cloned().collect::<Vec<_>>()
        );
    }

    #[test]
    fn xfast_range(items: BTreeSet<u32>) {
        let mut xfast = XFastSet::new();
        for item in items.iter() {
            xfast.insert(*item);
        }

        let min = items.iter().next().cloned().unwrap_or(0);
        let max = items
            .iter()
            .rev()
            .next()
            .cloned()
            .unwrap_or(u32::max_value());

        let mid = (max - min) / 2 + min;
        assert_eq!(
            xfast.range(min..max).collect::<Vec<_>>(),
            items.range(min..max).cloned().collect::<Vec<_>>(),
        );
        assert_eq!(
            xfast.range(min..mid).collect::<Vec<_>>(),
            items.range(min..mid).cloned().collect::<Vec<_>>(),
        );
        assert_eq!(
            xfast.range(mid..=max).collect::<Vec<_>>(),
            items.range(mid..=max).cloned().collect::<Vec<_>>(),
        );
    }

    #[test]
    fn xfast_predecessor_successor(items: HashSet<u16>) {
        let mut xfast = XFastSet::new();
        let mut items = Vec::from_iter(items.iter().map(|x| 2 * *x as u32));
        for item in items.iter() {
            xfast.insert(*item);
        }

        items.sort();
        for (i, item) in items.iter().cloned().enumerate() {
            assert_eq!(xfast.predecessor(item), Some(item));
            assert_eq!(xfast.successor(item), Some(item));

            if item > 0 {
                assert_eq!(xfast.successor(item - 1), Some(item));
                if i > 0 {
                    assert_eq!(xfast.predecessor(item - 1), Some(items[i - 1]));
                }
            }
            if item < u32::max_value() {
                assert_eq!(xfast.predecessor(item + 1), Some(item));
                if i < items.len() - 1 {
                    assert_eq!(xfast.successor(item + 1), Some(items[i + 1]));
                }
            }
        }
    }
}
