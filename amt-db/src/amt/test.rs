use super::{
    paring_provider::{Fr, Pairing},
    prove_params::AMTParams,
    tree::{AMTConfigTrait, AMTData, AMTree},
    trusted_setup::PP,
    utils::{DEPTHS, LENGTH},
};
use crate::storage::{FlattenArray, FlattenTree};
use algebra::{One, PairingEngine, PrimeField, Zero};
use std::{marker::PhantomData, sync::Arc};

impl<P: PrimeField> AMTData<P> for P {
    fn as_fr_int(&self) -> P::BigInt {
        self.clone().into()
    }

    fn as_fr(&self) -> P {
        self.clone()
    }
}

struct TestConfig<PE: PairingEngine> {
    _phantom: PhantomData<PE>,
}

impl<PE: PairingEngine> AMTConfigTrait for TestConfig<PE> {
    type PE = PE;
    type Name = String;
    type Data = Fr<PE>;
    type DataLayout = FlattenArray;
    type TreeLayout = FlattenTree;

    const DEPTHS: usize = DEPTHS;
}

type TestTree<PE> = AMTree<TestConfig<PE>>;

fn test_all<PE: PairingEngine>(
    amt: &mut TestTree<PE>,
    public_parameter: &AMTParams<PE>,
    task: &str,
) {
    // super::utils::type_hash::<PE>();
    for i in 0..TestConfig::<PE>::LENGTH {
        let proof = amt.prove(i);
        let value = amt.get(i);

        assert!(
            TestTree::verify(i, *value, amt.commitment(), proof, public_parameter),
            "fail at task {} pos {}",
            task,
            i
        );
    }
}

#[test]
fn test_amt() {
    let db = crate::storage::open_col("./__test_amt", 0);

    const DEPTHS: usize = TestConfig::<Pairing>::DEPTHS;

    let pp = PP::<Pairing>::from_file_or_new("./pp", DEPTHS);
    let pp = Arc::new(AMTParams::<Pairing>::from_pp(pp, DEPTHS));

    let mut amt = TestTree::<Pairing>::new("test".to_string(), db, pp.clone());

    test_all(&mut amt, &pp, "Empty");

    *amt.write(0) += Fr::<Pairing>::one();
    assert_eq!(amt.get(0), &Fr::<Pairing>::one());
    assert_eq!(amt.get(1), &Fr::<Pairing>::zero());
    test_all(&mut amt, &pp, "one-hot");

    *amt.write(0) += Fr::<Pairing>::one();
    *amt.write(LENGTH / 2) += Fr::<Pairing>::one();
    test_all(&mut amt, &pp, "sibling pair");

    ::std::fs::remove_dir_all("./__test_amt").unwrap();
}
