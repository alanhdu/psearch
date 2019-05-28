use std::collections::BTreeSet;

use proptest::prelude::*;
use bytemap::ByteMap;

proptest! {
    #[test]
    #[ignore]
    fn proptest_bytemap_insertions(
        keys in prop::collection::vec(any::<u8>(), 1..100)
    ) {
        let mut map = ByteMap::new();
        let mut expected = BTreeSet::new();
        for key in keys.iter().cloned() {
            assert_eq!(map.insert(key, key).is_some(), !expected.insert(key));
            assert_eq!(map.len(), expected.len());

            for i in 0..=255 {
                assert_eq!(
                    map.successor(i),
                    expected.range(i..).next().map(|k| (*k, k))
                );
                assert_eq!(
                    map.successor_mut(i).map(|(k, v)| (k, *v)),
                    expected.range(i..).next().map(|k| (*k, *k))
                );
                assert_eq!(
                    map.predecessor(i),
                    expected.range(0..=i).next_back().map(|k| (*k, k))
                );
                assert_eq!(
                    map.predecessor_mut(i).map(|(k, v)| (k, *v)),
                    expected.range(0..=i).next_back().map(|k| (*k, *k))
                );
            }
        }
    }
}
