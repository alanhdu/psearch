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


    macro_rules! reference {
        ($rng: expr, true) => {
            &$rng
        };
        ($rng: expr, false) => {
            $rng
        };
    }

    macro_rules! gen {
        ($name: ident, $set: ty, $n: expr) => {
            let mut rng = SmallRng::from_seed([5; 16]);
            let mut $name = <$set>::new();
            while $name.len() < $n {
                $name.insert(rng.gen());
            }
        }
    }

    macro_rules! random_contains {
        ($set: ty, $ty: ident) => {{
            gen!(set100, $set, 100);
            gen!(set1000, $set, 1000);
            gen!(set10_000, $set, 10000);
            gen!(set100_000, $set, 100_000);
            gen!(set1_000_000, $set, 1_000_000);
            gen!(set10_000_000, $set, 10_000_000);
            move |b, &n| {
                let set = match n {
                    100 => &set100,
                    1000 => &set1000,
                    10_000 => &set10_000,
                    100_000 => &set100_000,
                    1_000_000 => &set1_000_000,
                    10_000_000 => &set10_000_000,
                    _ => unreachable!(),
                };
                let mut rng = SmallRng::from_seed([7; 16]);
                b.iter(|| black_box(set.contains(reference!(rng.gen(), $ty))));
            }
        }};
    }
    c.bench(
        "random_contains_u32",
        ParameterizedBenchmark::new(
            "XFastSet",
            random_contains!(XFastSet<u32>, false),
            vec![100, 1000, 10000, 100000, 1_000_000, 10_000_000],
        )
        .with_function("YFastSet", random_contains!(YFastSet<u32>, false))
        .with_function("BTreeSet", random_contains!(BTreeSet<u32>, true))
    );
    c.bench(
        "random_contains_u64",
        ParameterizedBenchmark::new(
            "XFastSet",
            random_contains!(XFastSet<u64>, false),
            vec![100, 1000, 10000, 100000, 1_000_000, 10_000_000],
        )
        .with_function("YFastSet", random_contains!(YFastSet<u64>, false))
        .with_function("BTreeSet", random_contains!(BTreeSet<u64>, true))
    );

    macro_rules! successor {
        ($set: ident, $key: expr, true) => {
            $set.range($key..).next()
        };
        ($set: ident, $key: expr, false) => {
            $set.successor($key)
        };
    }
    macro_rules! random_successor {
        ($set: ty, $ty: ident) => {{
            gen!(set100, $set, 100);
            gen!(set1000, $set, 1000);
            gen!(set10_000, $set, 10000);
            gen!(set100_000, $set, 100_000);
            gen!(set1_000_000, $set, 1_000_000);
            gen!(set10_000_000, $set, 10_000_000);
            move |b, &n| {
                let set = match n {
                    100 => &set100,
                    1000 => &set1000,
                    10_000 => &set10_000,
                    100_000 => &set100_000,
                    1_000_000 => &set1_000_000,
                    10_000_000 => &set10_000_000,
                    _ => unreachable!(),
                };
                let mut rng = SmallRng::from_seed([7; 16]);
                b.iter(|| {
                    // This is a hack for type inference to work automatically
                    // Rust should optimize out the `if false` branch
                    let key = rng.gen();
                    if false {
                        set.contains(reference!(key, $ty));
                    }
                    black_box(successor!(set, key, $ty))
                });
            }
        }};
    }
    c.bench(
        "random_successor_u32",
        ParameterizedBenchmark::new(
            "XFastSet",
            random_successor!(XFastSet<u32>, false),
            vec![100, 1000, 10000, 100000, 1_000_000, 10_000_000],
        )
        .with_function("YFastSet", random_successor!(YFastSet<u32>, false))
        .with_function("BTreeSet", random_successor!(BTreeSet<u32>, true))
    );
    c.bench(
        "random_successor_u64",
        ParameterizedBenchmark::new(
            "XFastSet",
            random_successor!(XFastSet<u64>, false),
            vec![100, 1000, 10000, 100000, 1_000_000, 10_000_000],
        )
        .with_function("YFastSet", random_successor!(YFastSet<u64>, false))
        .with_function("BTreeSet", random_successor!(BTreeSet<u64>, true))
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
