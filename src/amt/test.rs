use super::prove_params::{Bls12_381_AMTPP, PUBLIC_PARAMETERS};
use super::{AMTParams, AMTree, LENGTH};
use algebra::bls12_381::{Bls12_381, Fr};
use algebra::{One, PairingEngine};

fn test_all<PE: PairingEngine, PP>(amt: &mut AMTree<PE>, public_parameter: &PP, task: &str)
where
    PP: AMTParams<PE>,
{
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
    let db = crate::db::open_db("./__test_amt", 0u32);

    let mut amt = AMTree::<Bls12_381>::new("test".to_string(), db);
    let pp: &Bls12_381_AMTPP = &PUBLIC_PARAMETERS;
    test_all(&mut amt, pp, "Empty");

    amt.inc(0, Fr::one(), pp);
    test_all(&mut amt, pp, "one-hot");

    amt.inc(0, Fr::one(), pp);
    amt.inc(LENGTH / 2, Fr::one(), pp);
    test_all(&mut amt, pp, "sibling pair");

    ::std::fs::remove_dir_all("./__test_amt").unwrap();
}
