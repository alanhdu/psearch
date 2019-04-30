use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::rc::Rc;

use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};
use rand::{Rng, SeedableRng};

use psearch::succinct::SloudsTrie;

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);

    let mut btree = BTreeMap::new();
    while btree.len() < 1_000_00 {
        btree.insert(rng.gen::<[u8; 8]>(), rng.gen::<u64>());
    }
    let btree = Rc::new(btree);
    let btree1 = Rc::clone(&btree);
    let btree2 = Rc::clone(&btree);

    c.bench(
        "get_8_bytes",
        ParameterizedBenchmark::new(
            "SloudsTrie",
            move |b, &i| {
                let slouds = SloudsTrie::from_iter(btree1.iter().take(i));
                let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
                b.iter(|| {
                    let needle = rng.gen::<[u8; 8]>();
                    black_box(slouds.get(&needle))
                });
            },
            vec![100, 1000, 10_000, 100_000, 1_000_000],
        )
        .with_function("BTree", move |b, &i| {
            let bt: BTreeMap<&[u8], u64> = BTreeMap::from_iter(
                btree2.iter().take(i).map(|(k, v)| (k as &[u8], *v)),
            );
            let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
            b.iter(|| {
                let needle = rng.gen::<[u8; 8]>();
                black_box(bt.get(&needle as &[u8]))
            });
        }),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
