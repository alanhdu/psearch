use std::iter::FromIterator;

use proptest::prelude::*;
use succinct::louds::{LoudsTrie, SLoudsTrie};

proptest! {
    #[test]
    #[ignore]
    fn proptest_loudstrie_u64(
        values in prop::collection::vec(
            any::<u64>(), 1..10_000
        ),
    ) {
        let mut louds = LoudsTrie::new();
        for key in values.iter() {
            louds.insert(key.to_be_bytes(), ());
        }
    }

    #[test]
    #[ignore]
    fn proptest_loudstrie_fixed_len(
        map in prop::collection::hash_map(
            any::<[u8; 8]>(), any::<u64>(), 1..10_000
        ),
        input: Vec<[u8; 8]>,
    ) {
        let mut louds = LoudsTrie::new();
        for (key, value) in map.iter() {
            louds.insert(key, *value);
        }

        for (key, value) in map.iter() {
            prop_assert_eq!(louds.get(key), Some(value));
        }

        for key in input.iter() {
            prop_assert_eq!(louds.get(key), map.get(key));
        }
    }

    #[test]
    #[ignore]
    fn proptest_loudstrie_variable_length(
        inputs in prop::collection::hash_set(any::<Vec<u8>>(), 1..1000),
        keys: Vec<Vec<u8>>,
    ) {
        let louds = LoudsTrie::from_iter(
            inputs.iter().map(|i| (i as &[u8], i.len()))
        );

        for input in inputs.iter() {
            let len = input.len();
            prop_assert_eq!(louds.get(input), Some(&len));
        }

        for key in keys.iter() {
            if inputs.contains(key) {
                let len = key.len();
                prop_assert_eq!(louds.get(key), Some(&len));
            } else {
                prop_assert_eq!(louds.get(key), None);
            }
        }
    }

    #[test]
    #[ignore]
    fn proptest_sloudstrie_fixed_len(
        map in prop::collection::hash_map(
            any::<[u8; 8]>(), any::<u64>(), 1..10000
        ),
        input: Vec<[u8; 8]>,
    ) {
        let slouds = SLoudsTrie::from_iter(map.clone());
        for key in map.keys() {
            prop_assert_eq!(slouds.get(key), map.get(key));
        }

        for key in input.iter() {
            prop_assert_eq!(slouds.get(key), map.get(key));
        }
    }

    #[test]
    #[ignore]
    fn proptest_sloudstrie_prefix(
        input in prop::collection::vec(any::<u8>(), 1..1000),
    ) {
        let slouds = SLoudsTrie::from_iter(
            (0..input.len()).map(|i| (&input[..i], i))
        );

        for i in 0..input.len() {
            prop_assert_eq!(slouds.get(&input[..i]), Some(&i));
        }
    }

    #[test]
    #[ignore]
    fn proptest_sloudstrie_variable_length(
        inputs in prop::collection::hash_set(any::<Vec<u8>>(), 1..1000),
        keys: Vec<Vec<u8>>,
    ) {
        let slouds = SLoudsTrie::from_iter(
            inputs.iter().map(|i| (i as &[u8], i.len()))
        );

        for input in inputs.iter() {
            let len = input.len();
            prop_assert_eq!(slouds.get(input), Some(&len));
        }

        for key in keys.iter() {
            if inputs.contains(key) {
                let len = key.len();
                prop_assert_eq!(slouds.get(key), Some(&len));
            } else {
                prop_assert_eq!(slouds.get(key), None);
            }
        }
    }
}
