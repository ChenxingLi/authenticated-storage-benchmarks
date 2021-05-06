use super::tree::{AMTConfigTrait, AMTData, AMTree};
use crate::crypto::export::FrInt;
use crate::crypto::{
    export::{CanonicalDeserialize, CanonicalSerialize, Fr, Pairing},
    AMTParams, PowerTau, TypeUInt,
};
use crate::impl_storage_from_canonical;
use crate::storage::{FlattenArray, FlattenTree, StorageDecodable, StorageEncodable};
use crate::type_uint;
use std::sync::Arc;

struct TestConfig {}

type_uint! {
    struct TestDepths(6);
}

impl_storage_from_canonical!(u64);

impl AMTConfigTrait for TestConfig {
    type PE = Pairing;
    type Name = u64;
    type Data = u64;
    type DataLayout = FlattenArray;
    type TreeLayout = FlattenTree;
    type Height = TestDepths;
}

type TestTree = AMTree<TestConfig>;

fn test_all(amt: &mut TestTree, public_parameter: &AMTParams<Pairing>, task: &str) {
    for i in 0..TestConfig::LENGTH {
        let proof = amt.prove(i);
        let value = amt.get(i);

        assert!(
            TestTree::verify(i, value.as_fr(), amt.commitment(), proof, public_parameter),
            "fail at task {} pos {}",
            task,
            i
        );
    }
}

impl AMTData<Fr<Pairing>> for u64 {
    fn as_fr_int(&self) -> FrInt<Pairing> {
        FrInt::<Pairing>::from(*self)
    }
}

#[test]
fn test_amt() {
    let db = crate::storage::open_col("./__test_amt", 0);

    const DEPTHS: usize = TestConfig::DEPTHS;
    const LENGTH: usize = 1 << DEPTHS;

    let pp = PowerTau::<Pairing>::from_file_or_new("./pp", DEPTHS);
    let pp = Arc::new(AMTParams::<Pairing>::from_pp(pp, DEPTHS));

    let mut amt = TestTree::new(64, db, pp.clone());

    test_all(&mut amt, &pp, "Empty");

    *amt.write(0) += 1;
    assert_eq!(amt.get(0), &1);
    assert_eq!(amt.get(1), &0);
    test_all(&mut amt, &pp, "one-hot");

    *amt.write(0) += &1;
    *amt.write(LENGTH / 2) += &1;
    test_all(&mut amt, &pp, "sibling pair");

    ::std::fs::remove_dir_all("./__test_amt").unwrap();
}
