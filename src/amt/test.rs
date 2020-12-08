use super::{
    paring_provider::{Fr, Pairing},
    prove_params::AMTParams,
    utils::{DEPTHS, LENGTH},
    AMTree,
};
use crate::amt::trusted_setup::PP;
use algebra::{One, PairingEngine};

fn test_all<PE: PairingEngine>(amt: &mut AMTree<PE>, public_parameter: &AMTParams<PE>, task: &str) {
    // super::utils::type_hash::<PE>();
    for i in 0..LENGTH {
        let proof = amt.prove(i);
        let value = amt.get(i);

        assert!(
            AMTree::<PE>::verify(i, *value, amt.commitment(), proof, public_parameter),
            "fail at task {} pos {}",
            task,
            i
        );
    }
}

#[test]
fn test_amt() {
    let db = crate::db::open_database("./__test_amt");

    let mut amt = AMTree::<Pairing>::new("test".to_string(), db);
    let pp = PP::<Pairing>::from_file_or_new("./pp", DEPTHS);
    let pp = &AMTParams::<Pairing>::from_pp(pp, DEPTHS);
    test_all(&mut amt, pp, "Empty");

    amt.inc(0, Fr::<Pairing>::one(), pp);
    test_all(&mut amt, pp, "one-hot");

    amt.inc(0, Fr::<Pairing>::one(), pp);
    amt.inc(LENGTH / 2, Fr::<Pairing>::one(), pp);
    test_all(&mut amt, pp, "sibling pair");

    ::std::fs::remove_dir_all("./__test_amt").unwrap();
}
