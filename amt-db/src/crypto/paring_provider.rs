use algebra::{bls12_381::Bls12_381, PairingEngine, PrimeField};

pub type G1<PE> = <PE as PairingEngine>::G1Projective;
pub type G1Aff<PE> = <PE as PairingEngine>::G1Affine;
pub type G2<PE> = <PE as PairingEngine>::G2Projective;
pub type G2Aff<PE> = <PE as PairingEngine>::G2Affine;
pub type Fr<PE> = <PE as PairingEngine>::Fr;
pub type FrInt<PE> = <Fr<PE> as PrimeField>::BigInt;

pub type Pairing = Bls12_381;
