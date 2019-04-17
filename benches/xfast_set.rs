use std::collections::{BTreeSet, HashSet};
use std::rc::Rc;

use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};
use rand::{Rng, SeedableRng};

use psearch::xfast::XFastSet;

fn xfast_sorted_insert(n: u32) -> XFastSet {
    let mut set = XFastSet::new();
    for i in 0..n {
        set.insert(i);
    }
    set
}

fn xfast_random_insert(items: &[u32]) -> XFastSet {
    let mut set = XFastSet::new();
    for i in items {
        set.insert(*i);
    }
    set
}

fn xfast_iter(set: &XFastSet) {
    for item in set.iter() {
        black_box(item);
    }
}

fn xfast_successor(set: &XFastSet, k: u32) -> Option<u32> {
    set.range(k..).next()
}

fn btree_sorted_insert(n: u32) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for i in 0..n {
        set.insert(i);
    }
    set
}

fn btree_random_insert(items: &[u32]) -> BTreeSet<u32> {
    let mut set = BTreeSet::new();
    for i in items {
        set.insert(*i);
    }
    set
}

fn btree_iter(set: &BTreeSet<u32>) {
    for item in set.iter() {
        black_box(item);
    }
}

fn btree_successor(set: &BTreeSet<u32>, k: u32) -> Option<u32> {
    set.range(k..).next().cloned()
}

fn hash_sorted_insert(n: u32) -> HashSet<u32> {
    let mut set = HashSet::with_capacity(n as usize);
    for i in 0..n {
        set.insert(i);
    }
    set
}

fn hash_random_insert(items: &[u32]) -> HashSet<u32> {
    let mut set = HashSet::new();
    for i in items {
        set.insert(*i);
    }
    set
}

fn criterion_benchmark(c: &mut Criterion) {
    c.bench(
        "sorted_insert",
        ParameterizedBenchmark::new(
            "XFastSet",
            |b, i| b.iter(|| black_box(xfast_sorted_insert(*i))),
            vec![100, 1000, 10000],
        )
        .with_function("BTreeSet", |b, i| {
            b.iter(|| black_box(btree_sorted_insert(*i)))
        })
        .with_function("HashSet", |b, i| {
            b.iter(|| black_box(hash_sorted_insert(*i)))
        }),
    );

    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
    let keys = (0..1_000_000).map(|_| rng.gen()).collect::<Vec<_>>();
    let keys = Rc::new(keys);
    let keys1 = keys.clone();
    let keys2 = keys.clone();
    let keys3 = keys.clone();

    c.bench(
        "random_insert",
        ParameterizedBenchmark::new(
            "XFastSet",
            move |b, &i| b.iter(|| black_box(xfast_random_insert(&keys1[..i]))),
            vec![100, 1000, 10000],
        )
        .with_function("BTreeSet", move |b, &i| {
            b.iter(|| black_box(btree_random_insert(&keys2[..i])))
        })
        .with_function("HashSet", move |b, &i| {
            b.iter(|| black_box(hash_random_insert(&keys3[..i])))
        }),
    );

    // Random Iteration
    let mut set = BTreeSet::new();
    while set.len() < 1_000_000 {
        set.insert(rng.gen::<u32>());
    }
    let unique_keys = Rc::new(set.iter().cloned().collect::<Vec<_>>());
    let unique_keys1 = unique_keys.clone();
    let unique_keys2 = unique_keys.clone();

    c.bench(
        "iter",
        ParameterizedBenchmark::new(
            "XFastSet",
            move |b, &i| {
                let mut set = XFastSet::new();
                for k in unique_keys1[..i].iter() {
                    set.insert(*k);
                }
                b.iter(|| black_box(xfast_iter(&set)))
            },
            vec![100, 1000, 10_000, 100_000],
        )
        .with_function("BTreeSet", move |b, &i| {
            let mut set = BTreeSet::new();
            for k in unique_keys2[..i].iter() {
                set.insert(*k);
            }
            b.iter(|| black_box(btree_iter(&set)))
        }),
    );

    // Successor Search
    let unique_keys1 = unique_keys.clone();
    let unique_keys2 = unique_keys.clone();

    c.bench(
        "successor_search",
        ParameterizedBenchmark::new(
            "XFastSet",
            move |b, &i| {
                let mut set = XFastSet::new();
                for k in unique_keys1[..i].iter() {
                    set.insert(*k);
                }
                let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
                b.iter(|| {
                    let needle = rng.gen();
                    black_box(xfast_successor(&set, needle))
                })
            },
            vec![100, 1000, 10_000, 100_000, 1_000_000],
        )
        .with_function("BTreeSet", move |b, &i| {
            let mut set = BTreeSet::new();
            for k in unique_keys2[..i].iter() {
                set.insert(*k);
            }
            let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);
            b.iter(|| {
                let needle = rng.gen();
                black_box(btree_successor(&set, needle))
            })
        }),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
