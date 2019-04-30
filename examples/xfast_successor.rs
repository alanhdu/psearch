use rand::{Rng, SeedableRng};

use criterion::black_box;
use psearch::xfast::XFastSet;

fn main() {
    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
    let keys = (0..10_000_000).map(|_| rng.gen()).collect::<Vec<_>>();

    let mut xfast = XFastSet::new();
    for key in &keys {
        xfast.insert(*key);
    }

    for _ in 0..20 {
        for key in &keys {
            black_box(xfast.successor(*key));
        }
    }
}
