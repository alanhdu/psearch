use std::collections::HashMap;
use std::iter::FromIterator;

use criterion::black_box;
use rand::{Rng, SeedableRng};
use structopt::StructOpt;

use psearch::{
    succinct::{LoudsTrie, SloudsTrie},
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

    #[structopt(name = "yfast_insert")]
    YFastInsert,
    #[structopt(name = "yfast_successor")]
    YFastSuccessor,

    #[structopt(name = "louds_insert")]
    LoudsInsert {
        #[structopt(short = "s", long = "string_size")]
        string_size: Option<u8>,
    },

    #[structopt(name = "louds_get")]
    LoudsGet {
        #[structopt(short = "s", long = "string_size")]
        string_size: Option<u8>,
    },

    #[structopt(name = "slouds_insert")]
    SLoudsInsert {
        #[structopt(short = "s", long = "string_size")]
        string_size: Option<u8>,
    },

    #[structopt(name = "slouds_get")]
    SLoudsGet {
        #[structopt(short = "s", long = "string_size")]
        string_size: Option<u8>,
    },
}

fn unique_hashmap<'a, 'b, R: Rng>(
    bytes: &'a [u8],
    rng: &'b mut R,
    byte_size: Option<u8>,
    size: usize,
) -> HashMap<&'a [u8], u64> {
    let mut map: HashMap<&[u8], u64> = HashMap::with_capacity(size);
    let mut cursor = 0;
    while map.len() < size {
        let size = byte_size.unwrap_or(rng.gen()) as usize;
        map.insert(&bytes[cursor..cursor + size], rng.gen());
        cursor += size;
    }
    map
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

        Ty::LoudsInsert { string_size } => {
            let max_size = string_size.unwrap_or(255) as usize;
            let mut bytes: Vec<u8> = vec![0; profile.size * max_size];
            rng.fill(&mut bytes as &mut [u8]);

            let mut values = Vec::with_capacity(profile.size);
            let mut cursor = 0;
            for _ in 0..profile.size {
                let size = string_size.unwrap_or(rng.gen()) as usize;
                values.push((&bytes[cursor..cursor + size], 0xdeadbeefu64));
                cursor += size;
            }

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                black_box(LoudsTrie::from_iter(values.iter().cloned()));
            }
        }

        Ty::LoudsGet { string_size } => {
            let max_size = string_size.unwrap_or(255) as usize;
            // Allocate some extra bytes in case of collisions
            let mut bytes: Vec<u8> = vec![0; profile.size * 5 * max_size / 4];
            rng.fill(&mut bytes as &mut [u8]);

            let louds = LoudsTrie::from_iter(unique_hashmap(
                &bytes,
                &mut rng,
                string_size,
                profile.size,
            ));

            dbg!("PROFILING");
            let mut needle: Vec<u8> = vec![0; max_size];
            for _ in 0..profile.iters {
                rng.fill(&mut needle as &mut [u8]);
                let size = rng.gen::<u8>();
                black_box(louds.get(&needle[..size as usize]));
            }
        }
        Ty::SLoudsInsert { string_size } => {
            let max_size = string_size.unwrap_or(255) as usize;
            let mut bytes: Vec<u8> = vec![0; profile.size * max_size];
            rng.fill(&mut bytes as &mut [u8]);

            let mut values = Vec::with_capacity(profile.size);
            let mut cursor = 0;
            for _ in 0..profile.size {
                let size = string_size.unwrap_or(rng.gen()) as usize;
                values.push((&bytes[cursor..cursor + size], 0xdeadbeefu64));
                cursor += size;
            }

            dbg!("PROFILING");
            for _ in 0..profile.iters {
                black_box(SloudsTrie::from_iter(values.iter().cloned()));
            }
        }

        Ty::SLoudsGet { string_size } => {
            let max_size = string_size.unwrap_or(255) as usize;
            // Allocate some extra bytes in case of collisions
            let mut bytes: Vec<u8> = vec![0; profile.size * 5 * max_size / 4];
            rng.fill(&mut bytes as &mut [u8]);

            let trie = SloudsTrie::from_iter(unique_hashmap(
                &bytes,
                &mut rng,
                string_size,
                profile.size,
            ));

            dbg!("PROFILING");
            let mut needle: Vec<u8> = vec![0; max_size];
            for _ in 0..profile.iters {
                rng.fill(&mut needle as &mut [u8]);
                let size = rng.gen::<u8>();
                black_box(trie.get(&needle[..size as usize]));
            }
        }
    }
}
