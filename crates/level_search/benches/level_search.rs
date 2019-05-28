use std::collections::{BTreeSet, HashSet};

use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use psearch::{xfast::XFastSet, yfast::YFastSet};

fn criterion_benchmark(c: &mut Criterion) {
    macro_rules! sorted_insert {
        ($x:ty) => {{
            |b, &n| {
                b.iter(|| {
                    let mut set = <$x>::new();
                    for i in 0..n {
                        set.insert(i);
                    }
                    black_box(set);
                })
            }
        }};
    }
    c.bench(
        "sorted_insert_u32",
        ParameterizedBenchmark::new(
            "XFastSet",
            sorted_insert!(XFastSet<u32>),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("YFastSet", sorted_insert!(YFastSet<u32>))
        .with_function("BTreeSet", sorted_insert!(BTreeSet<u32>))
        .with_function("HashSet", sorted_insert!(HashSet<u32>))
        .sample_size(10),
    );
    c.bench(
        "sorted_insert_u64",
        ParameterizedBenchmark::new(
            "XFastSet",
            sorted_insert!(XFastSet<u64>),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("YFastSet", sorted_insert!(YFastSet<u64>))
        .with_function("BTreeSet", sorted_insert!(BTreeSet<u64>))
        .with_function("HashSet", sorted_insert!(HashSet<u64>))
        .sample_size(10),
    );

    macro_rules! random_insert {
        ($x: ty) => {{
            |b, &n| {
                let mut rng = SmallRng::from_seed([5; 16]);
                let keys = (0..n).map(|_| rng.gen()).collect::<Vec<_>>();

                b.iter(|| {
                    let mut set = <$x>::new();
                    for key in keys.iter() {
                        set.insert(*key);
                    }
                    black_box(set);
                });
            }
        }};
    }
    c.bench(
        "random_insert_u32",
        ParameterizedBenchmark::new(
            "XFastSet",
            random_insert!(XFastSet<u32>),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("YFastSet", random_insert!(YFastSet<u32>))
        .with_function("BTreeSet", random_insert!(BTreeSet<u32>))
        .with_function("HashSet", random_insert!(HashSet<u32>))
        .sample_size(10),
    );
    c.bench(
        "random_insert_u64",
        ParameterizedBenchmark::new(
            "XFastSet",
            random_insert!(XFastSet<u64>),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("YFastSet", random_insert!(YFastSet<u64>))
        .with_function("BTreeSet", random_insert!(BTreeSet<u64>))
        .with_function("HashSet", random_insert!(HashSet<u64>))
        .sample_size(10),
    );

    macro_rules! bench_construct {
        ($ty: ty, $len: expr, ($set: ident, $b: ident) => $e: expr) => {{
            let sets = $len
                .iter()
                .map(|n| {
                    let mut rng = SmallRng::from_seed([5; 16]);
                    let mut set = <$ty>::new();
                    while set.len() < *n {
                        set.insert(rng.gen());
                    }
                    set
                })
                .collect::<Vec<_>>();
            move |$b, n| {
                let pos = $len.binary_search(n).unwrap();
                let $set = &sets[pos];
                $e
            }
        }};
    }
    let lens = [100, 1000, 10000, 100000, 1_000_000, 10_000_000];
    c.bench(
        "random_contains_u32",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u32>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.contains(rng.gen())));
            }),
            lens.to_vec(),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u32>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.contains(&rng.gen())));
            }),
        )
        .with_function(
            "HashSet",
            bench_construct!(HashSet<u32>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.contains(&rng.gen())));
            }),
        ),
    );
    c.bench(
        "contains_u32",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u32>, lens, (set, b) => {
                b.iter(|| black_box(set.contains(0xdeadbeef)));
            }),
            lens.to_vec(),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u32>, lens, (set, b) => {
                b.iter(|| black_box(set.contains(&0xdeadbeef)));
            }),
        )
        .with_function(
            "HashSet",
            bench_construct!(HashSet<u32>, lens, (set, b) => {
                b.iter(|| black_box(set.contains(&0xdeadbeef)));
            }),
        ),
    );

    c.bench(
        "random_contains_u64",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u64>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.contains(rng.gen())));
            }),
            lens.to_vec(),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u64>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.contains(&rng.gen())));
            }),
        )
        .with_function(
            "HashSet",
            bench_construct!(HashSet<u64>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.contains(&rng.gen())));
            }),
        ),
    );
    c.bench(
        "contains_u64",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u64>, lens, (set, b) => {
                b.iter(|| black_box(set.contains(0xdeadbeef)));
            }),
            lens.to_vec(),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u64>, lens, (set, b) => {
                b.iter(|| black_box(set.contains(&0xdeadbeef)));
            }),
        )
        .with_function(
            "HashSet",
            bench_construct!(HashSet<u64>, lens, (set, b) => {
                b.iter(|| black_box(set.contains(&0xdeadbeef)));
            }),
        ),
    );

    let lens = [100, 1000, 10000, 100000, 1_000_000];
    c.bench(
        "random_successor_u32",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u32>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.successor(rng.gen())));
            }),
            lens.to_vec(),
        )
        .with_function(
            "XFastSet",
            bench_construct!(XFastSet<u32>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.successor(rng.gen())));
            }),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u32>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.range(rng.gen::<u32>()..).next()));
            }),
        ),
    );
    c.bench(
        "successor_u32",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u32>, lens, (set, b) => {
                b.iter(|| black_box(set.successor(0xdeadbeef)));
            }),
            lens.to_vec(),
        )
        .with_function(
            "XFastSet",
            bench_construct!(XFastSet<u32>, lens, (set, b) => {
                b.iter(|| black_box(set.successor(0xdeadbeef)));
            }),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u32>, lens, (set, b) => {
                b.iter(|| black_box(set.range(0xdeadbeef..).next()));
            }),
        ),
    );

    c.bench(
        "random_successor_u64",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u64>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.successor(rng.gen())));
            }),
            lens.to_vec(),
        )
        .with_function(
            "XFastSet",
            bench_construct!(XFastSet<u64>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.successor(rng.gen())));
            }),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u64>, lens, (set, b) => {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| black_box(set.range(rng.gen::<u64>()..).next()));
            }),
        ),
    );
    c.bench(
        "successor_u64",
        ParameterizedBenchmark::new(
            "YFastSet",
            bench_construct!(YFastSet<u64>, lens, (set, b) => {
                b.iter(|| black_box(set.successor(0xdeadbeef)));
            }),
            lens.to_vec(),
        )
        .with_function(
            "XFastSet",
            bench_construct!(XFastSet<u64>, lens, (set, b) => {
                b.iter(|| black_box(set.successor(0xdeadbeef)));
            }),
        )
        .with_function(
            "BTreeSet",
            bench_construct!(BTreeSet<u64>, lens, (set, b) => {
                b.iter(|| black_box(set.range(0xdeadbeef..).next()));
            }),
        ),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
