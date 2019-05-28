use proptest::prelude::*;
use succinct::select_rank::{BitVec, SelectRank};

proptest! {
    #[test]
    #[ignore]
    fn proptest_bitvec_insert(input
        in prop::collection::vec(any::<(bool, usize)>(), 1..100_000)
    ) {
        let mut expected = Vec::with_capacity(input.len());
        let mut bits = BitVec::new();

        for (bit, order) in input.iter().cloned() {
            let order = order % (expected.len() + 1);
            bits.insert(order, bit);
            expected.insert(order, bit);
        }

        let mut n_ones = 0;
        let mut n_zeros = 0;
        for (i, bit) in expected.iter().cloned().enumerate() {
            prop_assert_eq!(bits.rank0(i), n_zeros);
            prop_assert_eq!(bits.rank1(i), n_ones);

            prop_assert_eq!(bits.get_bit(i), bit);

            n_zeros += !bit as usize;
            n_ones += bit as usize;
        }
        prop_assert_eq!(n_zeros, bits.len() - bits.num_ones() as usize);
        prop_assert_eq!(n_ones, bits.num_ones() as usize);

        for i in 0..n_zeros {
            prop_assert_eq!(expected[bits.select0(i)], false);
        }
        for i in 0..n_ones {
            prop_assert_eq!(expected[bits.select1(i)], true);
        }
    }
}
