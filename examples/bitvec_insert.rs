use criterion::black_box;
use psearch::select_rank::BitVec;

fn main() {
    let mut bits = BitVec::new();
    for i in 0..20_000_000 {
        bits.insert(i, true);
        bits.insert(i, false);
    }

    dbg!(&bits.len());
    black_box(bits);
}
