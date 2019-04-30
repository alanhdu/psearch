use rand::{Rng, SeedableRng};

use criterion::black_box;
use psearch::select_rank::{BitVec, SelectRank};

fn main() {
    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
    let keys = (0..10_000_000).map(|_| rng.gen()).collect::<Vec<_>>();

    let mut bits = BitVec::new();
    for key in &keys {
        bits.insert(0, *key);
    }

    for _ in 0..1_000_000_000 {
        black_box(bits.rank1(1_000_000));
    }
}
