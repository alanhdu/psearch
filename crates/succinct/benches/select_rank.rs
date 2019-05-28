use std::iter::FromIterator;
use std::rc::Rc;

use criterion::{
    black_box, criterion_group, criterion_main, Criterion,
    ParameterizedBenchmark,
};
use rand::{rngs::SmallRng, Rng, SeedableRng};

use succinct::select_rank::{BitVec, Bits512, SBitVec, SelectRank};

fn criterion_benchmark(c: &mut Criterion) {
    macro_rules! random_insert {
        ($ty: ty) => {{
            |b, &n| {
                let mut rng = SmallRng::from_seed([5; 16]);
                b.iter(|| {
                    let mut bits = <$ty>::new();
                    for _ in 0..n {
                        bits.insert(
                            rng.gen::<usize>() % (bits.len() + 1),
                            rng.gen::<bool>(),
                        );
                    }
                });
            }
        }};
    }
    c.bench(
        "insert_bits",
        ParameterizedBenchmark::new(
            "BitVec",
            random_insert!(BitVec),
            vec![100, 1000, 10000, 100000, 1000000],
        ),
    );
    c.bench(
        "insert_small_bits",
        ParameterizedBenchmark::new(
            "Bit512",
            random_insert!(Bits512),
            vec![10, 100, 512],
        ),
    );

    macro_rules! gen {
        ($ty: ty, $n: expr) => {{
            let mut rng = SmallRng::from_seed([5; 16]);
            let mut bits = [0u8; $n / 8 + 1];
            rng.fill(&mut bits as &mut [u8]);

            <$ty>::from_iter(
                bits.iter()
                    .flat_map(|b| (0..8).map(move |i| b & (1 << i) != 0))
                    .take($n),
            )
        }};
    }
    let bitvecs = Rc::new([
        gen!(BitVec, 1000),
        gen!(BitVec, 10000),
        gen!(BitVec, 100000),
        gen!(BitVec, 1000000),
        gen!(BitVec, 10000000),
    ]);
    let sbitvecs = Rc::new([
        gen!(SBitVec, 1000),
        gen!(SBitVec, 10000),
        gen!(SBitVec, 100000),
        gen!(SBitVec, 1000000),
        gen!(SBitVec, 10000000),
    ]);

    macro_rules! func {
        ($ty: ident, ($n: ident, $rng: ident, $name: ident) => $e: expr) => {{
            let inputs = Rc::clone(&$ty);
            move |b, &$n| {
                let $name = match $n {
                    1000 => &inputs[0],
                    10000 => &inputs[1],
                    100000 => &inputs[2],
                    1000000 => &inputs[3],
                    10000000 => &inputs[4],
                    _ => unreachable!(),
                };
                let mut $rng = SmallRng::from_seed([7; 16]);
                b.iter(|| black_box($e));
            }
        }};
    }

    c.bench(
        "rank0",
        ParameterizedBenchmark::new(
            "BitVec",
            func!(bitvecs, (n, rng, bits) => bits.rank0(rng.gen::<usize>() % n)),
            vec![1000, 10000, 100000, 1000000, 10000000],
        )
        .with_function(
            "SBitVec",
            func!(sbitvecs, (n, rng, bits) => bits.rank0(rng.gen::<usize>() % n)),
        ),
    );
    c.bench(
        "select0",
        ParameterizedBenchmark::new(
            "BitVec",
            func!(bitvecs, (n, rng, bits) =>
                  bits.select1(rng.gen::<usize>() % (n / 10))),
            vec![1000, 10000, 100000, 1000000, 10000000],
        )
        .with_function(
            "SBitVec",
            func!(sbitvecs, (n, rng, bits) =>
                  bits.select1(rng.gen::<usize>() % (n / 10))),
        ),
    );
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
