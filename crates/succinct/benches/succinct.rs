use std::collections::{BTreeMap, HashMap};
use std::io::Read;
use std::rc::Rc;

use bstr::BString;
use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};
use rand::{rngs::SmallRng, seq::SliceRandom, Rng, SeedableRng};

use succinct::louds::{LoudsTrie, SLoudsTrie};

fn criterion_benchmark(c: &mut Criterion) {
    macro_rules! sorted_insert {
        ($x:ty, $i: ident => $e: expr) => {{
            |b, &n| {
                b.iter(|| {
                    let mut map = <$x>::new();
                    for $i in 0..n {
                        map.insert($e, $i);
                    }
                    black_box(map);
                })
            }
        }};
    }
    c.bench(
        "insert_sorted_8_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            sorted_insert!(BTreeMap<u64, u64>, i => i),
            vec![100, 1000, 10000, 100000],
        )
        .with_function(
            "LoudsTrie",
            sorted_insert!(LoudsTrie<u64>, i => i.to_be_bytes()),
        )
        .with_function("HashMap", sorted_insert!(HashMap<u64, u64>, i => i)),
    );

    macro_rules! random_insert {
        ($map: ty, $key: ty) => {{
            |b, &n| {
                let mut rng = SmallRng::from_seed([5; 16]);
                let keys =
                    (0..n).map(|_| rng.gen::<$key>()).collect::<Vec<_>>();

                b.iter(|| {
                    let mut map = <$map>::new();
                    for (i, key) in keys.iter().cloned().enumerate() {
                        map.insert(key, i);
                    }
                    black_box(map);
                });
            }
        }};
    }
    c.bench(
        "insert_random_8_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            random_insert!(BTreeMap<u64, usize>, u64),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("LoudsTrie", random_insert!(LoudsTrie<usize>, [u8; 8])),
    );
    c.bench(
        "insert_random_32_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            random_insert!(BTreeMap<[u8; 32], usize>, [u8; 32]),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("LoudsTrie", random_insert!(LoudsTrie<usize>, [u8; 32])),
    );

    // 16 MB file
    let mut buffer = Vec::with_capacity(17_000_000);
    let mut urls = std::fs::File::open("benches/urls.csv").unwrap();
    urls.read_to_end(&mut buffer).unwrap();
    let bytes = BString::from_vec(buffer);
    let mut urls = bytes
        .lines()
        .map(|line| line.to_bstring().into_vec())
        .collect::<Vec<_>>();
    let mut rng = SmallRng::from_seed([5; 16]);
    urls.shuffle(&mut rng);
    let urls = Rc::new(urls);

    macro_rules! url_insert {
        ($map: ty) => {{
            let urls = Rc::clone(&urls);
            move |b, &n| {
                b.iter(|| {
                    let mut map = <$map>::new();
                    for (i, key) in urls.iter().take(n).enumerate() {
                        map.insert(&key, i);
                    }
                    black_box(map);
                });
            }
        }};
    }
    c.bench(
        "insert_urls",
        ParameterizedBenchmark::new(
            "BTree",
            url_insert!(BTreeMap<&[u8], usize>),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("LoudsTrie", url_insert!(LoudsTrie<usize>))
        .sample_size(10),
    );

    macro_rules! gen {
        ($map: ty, $n: expr, $rng: ident => $e: expr) => {{
            let mut $rng = SmallRng::from_seed([5; 16]);
            let mut map = <$map>::new();
            let mut count: usize = 0;
            while map.len() < $n {
                map.insert($e, count);
                count += 1;
            }
            map
        }};
    }
    macro_rules! random_get {
        ($name: ident, $rng: ident => $e: expr) => {{
            let rc = Rc::clone(&$name);
            move |b, &n| {
                let map = match n {
                    100 => &rc[0],
                    1000 => &rc[1],
                    10000 => &rc[2],
                    100000 => &rc[3],
                    1000000 => &rc[4],
                    _ => unimplemented!(),
                };
                let mut $rng = SmallRng::from_seed([7; 16]);
                b.iter(|| black_box(map.get($e)));
            }
        }};
    }

    let louds8 = Rc::new([
        gen!(LoudsTrie<usize>, 100, rng => rng.gen::<[u8; 8]>()),
        gen!(LoudsTrie<usize>, 1000, rng => rng.gen::<[u8; 8]>()),
        gen!(LoudsTrie<usize>, 10_000, rng => rng.gen::<[u8; 8]>()),
        gen!(LoudsTrie<usize>, 100_000, rng => rng.gen::<[u8; 8]>()),
        gen!(LoudsTrie<usize>, 1_000_000, rng => rng.gen::<[u8; 8]>()),
    ]);
    let slouds8 = Rc::new([
        SLoudsTrie::from(&louds8[0]),
        SLoudsTrie::from(&louds8[1]),
        SLoudsTrie::from(&louds8[2]),
        SLoudsTrie::from(&louds8[3]),
        SLoudsTrie::from(&louds8[4]),
    ]);
    let btree8 = Rc::new([
        gen!(BTreeMap<u64, usize>, 100, rng => rng.gen::<u64>()),
        gen!(BTreeMap<u64, usize>, 1000, rng => rng.gen::<u64>()),
        gen!(BTreeMap<u64, usize>, 10_000, rng => rng.gen::<u64>()),
        gen!(BTreeMap<u64, usize>, 100_000, rng => rng.gen::<u64>()),
        gen!(BTreeMap<u64, usize>, 1_000_000, rng => rng.gen::<u64>()),
    ]);
    c.bench(
        "get_random_8_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            random_get!(btree8, rng => &rng.gen::<u64>()),
            vec![100, 1000, 10000, 100000, 1_000_000],
        )
        .with_function(
            "LoudsTrie",
            random_get!(louds8, rng => rng.gen::<[u8; 8]>()),
        )
        .with_function(
            "SLoudsTrie",
            random_get!(slouds8, rng => rng.gen::<[u8; 8]>()),
        ),
    );

    let louds32 = Rc::new([
        gen!(LoudsTrie<usize>, 100, rng => rng.gen::<[u8; 32]>()),
        gen!(LoudsTrie<usize>, 1000, rng => rng.gen::<[u8; 32]>()),
        gen!(LoudsTrie<usize>, 10_000, rng => rng.gen::<[u8; 32]>()),
        gen!(LoudsTrie<usize>, 100_000, rng => rng.gen::<[u8; 32]>()),
        gen!(LoudsTrie<usize>, 1_000_000, rng => rng.gen::<[u8; 32]>()),
    ]);
    let btree32 = Rc::new([
        gen!(BTreeMap<[u8; 32], usize>, 100, rng => rng.gen::<[u8; 32]>()),
        gen!(BTreeMap<[u8; 32], usize>, 1000, rng => rng.gen::<[u8; 32]>()),
        gen!(BTreeMap<[u8; 32], usize>, 10_000, rng => rng.gen::<[u8; 32]>()),
        gen!(BTreeMap<[u8; 32], usize>, 100_000, rng => rng.gen::<[u8; 32]>()),
        gen!(BTreeMap<[u8; 32], usize>, 1_000_000,
             rng => rng.gen::<[u8; 32]>()),
    ]);
    let slouds32 = Rc::new([
        SLoudsTrie::from(&louds32[0]),
        SLoudsTrie::from(&louds32[1]),
        SLoudsTrie::from(&louds32[2]),
        SLoudsTrie::from(&louds32[3]),
        SLoudsTrie::from(&louds32[4]),
    ]);

    c.bench(
        "get_random_32_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            random_get!(btree32, rng => &rng.gen::<[u8; 32]>()),
            vec![100, 1000, 10000, 100000, 1_000_000],
        )
        .with_function(
            "LoudsTrie",
            random_get!(louds32, rng => rng.gen::<[u8; 32]>()),
        )
        .with_function(
            "SLoudsTrie",
            random_get!(slouds32, rng => rng.gen::<[u8; 32]>()),
        ),
    );

    fn generate(rng: &mut SmallRng) -> Vec<u8> {
        let mut s = vec![0; rng.gen::<usize>() % 100];
        rng.fill(&mut s as &mut [u8]);
        s
    }
    let louds_var = Rc::new([
        gen!(LoudsTrie<usize>, 100, rng => generate(&mut rng)),
        gen!(LoudsTrie<usize>, 1_000, rng => generate(&mut rng)),
        gen!(LoudsTrie<usize>, 10_000, rng => generate(&mut rng)),
        gen!(LoudsTrie<usize>, 100_000, rng => generate(&mut rng)),
        gen!(LoudsTrie<usize>, 1_000_000, rng => generate(&mut rng)),
    ]);

    let slouds_var = Rc::new([
        SLoudsTrie::from(&louds_var[0]),
        SLoudsTrie::from(&louds_var[1]),
        SLoudsTrie::from(&louds_var[2]),
        SLoudsTrie::from(&louds_var[3]),
        SLoudsTrie::from(&louds_var[4]),
    ]);

    let btree_var = Rc::new([
        gen!(BTreeMap<Vec<u8>, usize>, 100, rng => generate(&mut rng)),
        gen!(BTreeMap<Vec<u8>, usize>, 1_000, rng => generate(&mut rng)),
        gen!(BTreeMap<Vec<u8>, usize>, 10_000, rng => generate(&mut rng)),
        gen!(BTreeMap<Vec<u8>, usize>, 100_000, rng => generate(&mut rng)),
        gen!(BTreeMap<Vec<u8>, usize>, 1_000_000, rng => generate(&mut rng)),
    ]);
    c.bench(
        "get_random_var_0_to_100_bytes",
        ParameterizedBenchmark::new(
            "BTree",
            random_get!(btree_var, rng => &generate(&mut rng)),
            vec![100, 1000, 10000, 100000, 1_000_000],
        )
        .with_function(
            "LoudsTrie",
            random_get!(louds_var, rng => &generate(&mut rng)),
        )
        .with_function(
            "SLoudsTrie",
            random_get!(slouds_var, rng => &generate(&mut rng)),
        ),
    );

    macro_rules! url_gen {
        ($map: ty, $n: expr) => {{
            let mut urls = Vec::clone(&urls);
            let mut rng = SmallRng::from_seed([5; 16]);
            urls.shuffle(&mut rng);
            let mut map = <$map>::new();
            for (i, key) in urls.iter().take($n).enumerate() {
                map.insert(key.clone(), i);
            }
            map
        }};
    }

    let btree_url = Rc::new([
        url_gen!(BTreeMap<Vec<u8>, usize>, 100),
        url_gen!(BTreeMap<Vec<u8>, usize>, 1000),
        url_gen!(BTreeMap<Vec<u8>, usize>, 10000),
        url_gen!(BTreeMap<Vec<u8>, usize>, 100000),
        url_gen!(BTreeMap<Vec<u8>, usize>, 211708),
    ]);
    let louds_url = Rc::new([
        url_gen!(LoudsTrie<usize>, 100),
        url_gen!(LoudsTrie<usize>, 1000),
        url_gen!(LoudsTrie<usize>, 10000),
        url_gen!(LoudsTrie<usize>, 100000),
        url_gen!(LoudsTrie<usize>, 211708),
    ]);
    let slouds_url = Rc::new([
        SLoudsTrie::from(&louds_url[0]),
        SLoudsTrie::from(&louds_url[1]),
        SLoudsTrie::from(&louds_url[2]),
        SLoudsTrie::from(&louds_url[3]),
        SLoudsTrie::from(&louds_url[4]),
    ]);

    macro_rules! url_get {
        ($name: ident, $e: expr) => {{
            let rc = Rc::clone(&$name);
            let urls = Rc::clone(&urls);
            move |b, &n| {
                let map = match n {
                    100 => &rc[0],
                    1000 => &rc[1],
                    10000 => &rc[2],
                    100000 => &rc[3],
                    _ => unimplemented!(),
                };
                let mut rng = SmallRng::from_seed([7; 16]);
                b.iter(|| {
                    let index: usize = if $e {
                        rng.gen_range(0,n)
                    } else {
                        rng.gen_range(n, 211708)
                    };
                    black_box(map.get(&urls[index]));
                });
            }
        }};
    }

    c.bench(
        "get_random_url_true",
        ParameterizedBenchmark::new(
            "BTree",
            url_get!(btree_url, true),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("LoudsTrie", url_get!(louds_url, true))
        .with_function("SLoudsTrie", url_get!(slouds_url, true)),
    );
    c.bench(
        "get_random_url_false",
        ParameterizedBenchmark::new(
            "BTree",
            url_get!(btree_url, false),
            vec![100, 1000, 10000, 100000],
        )
        .with_function("LoudsTrie", url_get!(louds_url, false))
        .with_function("SLoudsTrie", url_get!(slouds_url, false)),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
