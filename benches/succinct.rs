use std::collections::BTreeMap;
use std::iter::FromIterator;
use std::rc::Rc;

use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};
use rand::{Rng, SeedableRng};

use psearch::succinct::{LoudsTrie, SLoudsTrie};

fn criterion_benchmark(c: &mut Criterion) {
    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
    let mut btree = BTreeMap::new();
    while btree.len() < 10_000_000 {
        btree.insert(rng.gen::<[u8; 8]>(), rng.gen::<u64>());
    }
    let btree = Rc::new(btree);
    let btree1 = Rc::clone(&btree);
    let btree2 = Rc::clone(&btree);

    c.bench(
        "get_random_8_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            move |b, &i| {
                let bt: BTreeMap<u64, u64> = BTreeMap::from_iter(
                    btree
                        .iter()
                        .take(i)
                        .map(|(k, v)| (u64::from_be_bytes(*k), *v)),
                );
                let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
                b.iter(|| {
                    let needle = rng.gen::<u64>();
                    black_box(bt.get(&needle));
                });
            },
            vec![100, 1000, 10_000, 100_000, 1_000_000, 10_000_000],
        )
        .with_function("Slouds", move |b, &i| {
            let slouds = SLoudsTrie::from_iter(btree1.iter().take(i));
            let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
            b.iter(|| {
                let needle = rng.gen::<[u8; 8]>();
                black_box(slouds.get(&needle))
            });
        })
        .with_function("Louds", move |b, &i| {
            let trie = LoudsTrie::from_iter(btree2.iter().take(i));
            let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
            b.iter(|| {
                let needle = rng.gen::<[u8; 8]>();
                black_box(trie.get(&needle))
            });
        }),
    );

    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
    let mut btree = BTreeMap::new();
    while btree.len() < 10_000_000 {
        btree.insert(rng.gen::<[u8; 32]>(), rng.gen::<u64>());
    }
    let btree = Rc::new(btree);
    let btree1 = Rc::clone(&btree);
    let btree2 = Rc::clone(&btree);
    c.bench(
        "get_random_32_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            move |b, &i| {
                let bt: BTreeMap<[u8; 32], u64> = BTreeMap::from_iter(
                    btree.iter().take(i).map(|(k, v)| (*k, *v)),
                );
                let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
                b.iter(|| {
                    let needle = rng.gen::<[u8; 32]>();
                    black_box(bt.get(&needle))
                });
            },
            vec![100, 1000, 10_000, 100_000, 1_000_000, 10_000_000],
        )
        .with_function("Slouds", move |b, &i| {
            // We current OoM on constructing the SLoudsTrie.
            // Need to improve BadTrie to improve this
            if i == 10_000_000 {
                return;
            }

            let slouds = SLoudsTrie::from_iter(btree1.iter().take(i));
            let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
            b.iter(|| {
                let needle = rng.gen::<[u8; 32]>();
                black_box(slouds.get(&needle))
            });
        })
        .with_function("Louds", move |b, &i| {
            let trie = LoudsTrie::from_iter(btree2.iter().take(i));
            let mut rng = rand::rngs::SmallRng::from_seed([7; 16]);
            b.iter(|| {
                let needle = rng.gen::<[u8; 32]>();
                black_box(trie.get(&needle))
            });
        }),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
