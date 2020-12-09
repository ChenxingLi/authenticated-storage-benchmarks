pub mod node;
pub mod paring_provider;
pub mod prove_params;
pub mod tree;
pub mod trusted_setup;
pub mod utils;

pub use self::{
    node::NodeIndex,
    tree::{AMTData, AMTree},
    utils::*,
};

#[cfg(test)]
mod test {
    use super::{
        paring_provider::{Fr, Pairing},
        prove_params::AMTParams,
        tree::{AMTData, AMTree},
        trusted_setup::PP,
        utils::{DEPTHS, LENGTH},
    };
    use algebra::{One, PairingEngine, PrimeField};

    type TestTree<PE> = AMTree<PE, Fr<PE>>;

    impl<P: PrimeField> AMTData<P> for P {
        fn as_fr_int(&self) -> P::BigInt {
            self.clone().into()
        }

        fn as_fr(&self) -> P {
            self.clone()
        }
    }

    fn test_all<PE: PairingEngine>(
        amt: &mut TestTree<PE>,
        public_parameter: &AMTParams<PE>,
        task: &str,
    ) {
        // super::utils::type_hash::<PE>();
        for i in 0..LENGTH {
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
        let db = crate::storage::open_database("./__test_amt");

        let inc_one = |x: &mut Fr<Pairing>| *x += Fr::<Pairing>::one();

        let mut amt = TestTree::<Pairing>::new("test".to_string(), db);
        let pp = PP::<Pairing>::from_file_or_new("./pp", DEPTHS);
        let pp = &AMTParams::<Pairing>::from_pp(pp, DEPTHS);
        test_all(&mut amt, pp, "Empty");

        amt.update(0, inc_one, pp);
        test_all(&mut amt, pp, "one-hot");

        amt.update(0, inc_one, pp);
        amt.update(LENGTH / 2, inc_one, pp);
        test_all(&mut amt, pp, "sibling pair");

        ::std::fs::remove_dir_all("./__test_amt").unwrap();
    }
}
