use std::collections::BTreeMap;
use std::iter::FromIterator;

use criterion::black_box;
use psearch::succinct::SLouds;
use rand::{Rng, SeedableRng};

fn main() {
    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);

    let btree = BTreeMap::from_iter(
        (0..100_000).map(|_| (rng.gen::<[u8; 8]>(), rng.gen::<u64>())),
    );

    let slouds = SLouds::from_iter(btree);

    let needle = 0xdeadbeefu64.to_be_bytes();
    for _ in 0..100_000_000 {
        black_box(slouds.get(needle));
    }
}
