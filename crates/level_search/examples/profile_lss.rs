use std::collections::BTreeSet;

use criterion::black_box;
use rand::{Rng, SeedableRng};
use structopt::StructOpt;

use level_search::{
    xfast::XFastSet,
    yfast::YFastSet,
};

#[derive(StructOpt)]
struct Profile {
    #[structopt(short = "s", long = "size", default_value = "1000000")]
    size: usize,
    #[structopt(short = "i", long = "iters")]
    iters: usize,
    #[structopt(subcommand)]
    ty: Ty,
}

#[derive(StructOpt)]
enum Ty {
    #[structopt(name = "xfast_insert")]
    XFastInsert,
    #[structopt(name = "xfast_successor")]
    XFastSuccessor,

    #[structopt(name = "btree_successor")]
    BTreeSuccessor,

    #[structopt(name = "yfast_insert")]
    YFastInsert,
    #[structopt(name = "yfast_successor")]
    YFastSuccessor,
}

fn main() {
    let profile = Profile::from_args();
    let mut rng = rand::rngs::SmallRng::from_seed([5; 16]);

    match profile.ty {
        Ty::XFastInsert => {
            let keys: Vec<u32> = (0..profile.size).map(|_| rng.gen()).collect();

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                let mut xfast = XFastSet::new();
                for key in &keys {
                    black_box(xfast.insert(*key));
                }
            }
        }
        Ty::XFastSuccessor => {
            let mut xfast: XFastSet<u32> = XFastSet::new();
            while xfast.len() < profile.size {
                xfast.insert(rng.gen());
            }

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                black_box(xfast.successor(rng.gen()));
            }
        }
        Ty::BTreeSuccessor => {
            let mut btree: BTreeSet<u32> = BTreeSet::new();
            while btree.len() < profile.size {
                btree.insert(rng.gen());
            }

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                black_box(btree.range(&rng.gen()..).next());
            }
        }
        Ty::YFastSuccessor => {
            let mut yfast: YFastSet<u32> = YFastSet::new();
            while yfast.len() < profile.size {
                yfast.insert(rng.gen());
            }

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                black_box(yfast.successor(rng.gen()));
            }
        }
        Ty::YFastInsert => {
            let keys: Vec<u32> = (0..profile.size).map(|_| rng.gen()).collect();

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                let mut yfast = YFastSet::new();
                for key in &keys {
                    black_box(yfast.insert(*key));
                }
            }
        }
    }
}
