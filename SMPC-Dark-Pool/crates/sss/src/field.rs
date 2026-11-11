use ark_bn254::Fr;
use ark_ff::{Field, UniformRand};

pub type ScalarField = Fr;

pub fn random_scalar(rng: &mut impl rand::RngCore) -> ScalarField {
    ScalarField::rand(rng)
}

pub fn random_nonzero_scalar(rng: &mut impl rand::RngCore) -> ScalarField {
    loop {
        let s = ScalarField::rand(rng);
        if s != ScalarField::ZERO {
            return s;
        }
    }
}
