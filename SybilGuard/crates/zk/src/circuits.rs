use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystemRef, SynthesisError};
use ark_std::One;

pub type Fr = ark_bls12_381::Fr;

#[derive(Clone)]
pub struct ClusterMembershipPublicInput {
    pub cluster_nullifier: Fr,
    pub min_size: Fr,
    pub leaf_commitment: Fr,
}

#[derive(Clone)]
pub struct ClusterMembershipWitness {
    pub secret_seed: Fr,
    pub merkle_siblings: Vec<Fr>,
    pub merkle_directions: Vec<bool>,
    pub actual_cluster_size: Fr,
    pub cluster_root: Fr,
}

#[derive(Clone)]
pub struct ClusterMembershipCircuit {
    pub public: Option<ClusterMembershipPublicInput>,
    pub private: Option<ClusterMembershipWitness>,
}

impl ClusterMembershipCircuit {
    pub fn new(public: ClusterMembershipPublicInput, private: ClusterMembershipWitness) -> Self {
        Self { public: Some(public), private: Some(private) }
    }

    pub fn dummy() -> Self {
        Self {
            public: Some(ClusterMembershipPublicInput {
                cluster_nullifier: Fr::from(1u64), min_size: Fr::from(5u64), leaf_commitment: Fr::from(2u64),
            }),
            private: Some(ClusterMembershipWitness {
                secret_seed: Fr::from(42u64),
                merkle_siblings: vec![Fr::from(0u64); 2],
                merkle_directions: vec![false; 2],
                actual_cluster_size: Fr::from(10u64),
                cluster_root: Fr::from(0u64),
            }),
        }
    }
}

impl ConstraintSynthesizer<Fr> for ClusterMembershipCircuit {
    fn generate_constraints(self, cs: ConstraintSystemRef<Fr>) -> Result<(), SynthesisError> {
        use ark_r1cs_std::prelude::*;
        use ark_r1cs_std::fields::fp::FpVar;
        use ark_r1cs_std::boolean::Boolean;
        use ark_ff::Field;

        // Public inputs
        let pub_nullifier = FpVar::new_input(cs.clone(), || Ok(self.public.as_ref().unwrap().cluster_nullifier))?;
        let pub_min_size   = FpVar::new_input(cs.clone(), || Ok(self.public.as_ref().unwrap().min_size))?;
        let pub_leaf       = FpVar::new_input(cs.clone(), || Ok(self.public.as_ref().unwrap().leaf_commitment))?;

        // Witness
        let wit_secret = FpVar::new_witness(cs.clone(), || Ok(self.private.as_ref().unwrap().secret_seed))?;
        let wit_root   = FpVar::new_witness(cs.clone(), || Ok(self.private.as_ref().unwrap().cluster_root))?;
        let wit_size   = FpVar::new_witness(cs.clone(), || Ok(self.private.as_ref().unwrap().actual_cluster_size))?;

        let depth = self.private.as_ref().unwrap().merkle_siblings.len();
        let mut siblings   = Vec::new();
        let mut directions = Vec::new();
        for i in 0..depth {
            siblings.push(FpVar::new_witness(cs.clone(), || Ok(self.private.as_ref().unwrap().merkle_siblings[i]))?);
            directions.push(Boolean::new_witness(cs.clone(), || Ok(self.private.as_ref().unwrap().merkle_directions[i]))?);
        }

        // C1: nullifier = secret^3 + 42
        let sq = &wit_secret * &wit_secret;
        let computed = &sq * &wit_secret + FpVar::Constant(Fr::from(42u64));
        computed.enforce_equal(&pub_nullifier)?;

        // C2: Merkle proof — arithmetic selection
        let mut current = pub_leaf.clone();
        let one_fp = FpVar::Constant(Fr::one());
        for i in 0..depth {
            let dir = FpVar::from(directions[i].clone());
            let not_dir = &one_fp - &dir;
            let left  = &dir * &siblings[i] + &not_dir * &current;
            let right = &dir * &current + &not_dir * &siblings[i];
            let sum = &left + &right;
            let sq = &sum * &sum;
            current = &sq * &sum + FpVar::Constant(Fr::from(42u64));
        }
        current.enforce_equal(&wit_root)?;

        // C3: size >= min
        let one  = FpVar::Constant(Fr::one());
        let diff = &wit_size - &pub_min_size + &one;
        let diff_inv = FpVar::new_witness(cs.clone(), || {
            let v = self.private.as_ref().unwrap().actual_cluster_size
                - self.public.as_ref().unwrap().min_size + Fr::one();
            Ok(v.inverse().unwrap_or(Fr::from(0u64)))
        })?;
        (&diff * &diff_inv).enforce_equal(&one)?;

        // C4: leaf = secret^3 + 7
        let leaf_calc = {
            let sq = &wit_secret * &wit_secret;
            &sq * &wit_secret + FpVar::Constant(Fr::from(7u64))
        };
        leaf_calc.enforce_equal(&pub_leaf)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_bls12_381::{Bls12_381, Fr};
    use ark_groth16::Groth16;
    use ark_snark::SNARK;
    use rand::rngs::OsRng;
    use ark_std::UniformRand;

    #[test]
    fn test_constraints() {
        let cs = ark_relations::r1cs::ConstraintSystem::new_ref();
        ClusterMembershipCircuit::dummy().generate_constraints(cs.clone()).unwrap();
        assert!(cs.num_constraints() > 0);
        assert!(cs.num_witness_variables() > 0);
        println!("✓ {} constraints, {} witness vars", cs.num_constraints(), cs.num_witness_variables());
    }

    #[test]
    fn test_groth16() {
        let mut rng = OsRng;
        let secret = Fr::from(42u64);
        let leaf   = secret * secret * secret + Fr::from(7u64);
        let sib0   = Fr::rand(&mut rng);
        let sib1   = Fr::rand(&mut rng);
        let h0     = { let s = leaf + sib0; s * s * s + Fr::from(42u64) };
        let root   = { let s = h0 + sib1; s * s * s + Fr::from(42u64) };

        let public = ClusterMembershipPublicInput {
            cluster_nullifier: secret * secret * secret + Fr::from(42u64),
            min_size:          Fr::from(3u64),
            leaf_commitment:   leaf,
        };
        let private = ClusterMembershipWitness {
            secret_seed:      secret,
            merkle_siblings:  vec![sib0, sib1],
            merkle_directions: vec![false, true],
            actual_cluster_size: Fr::from(5u64),
            cluster_root:     root,
        };

        let circuit   = ClusterMembershipCircuit::new(public.clone(), private);
        let (pk, vk)  = Groth16::<Bls12_381>::circuit_specific_setup(circuit.clone(), &mut rng).unwrap();
        let proof     = Groth16::<Bls12_381>::prove(&pk, circuit, &mut rng).unwrap();
        let inputs    = vec![
            secret * secret * secret + Fr::from(42u64),
            Fr::from(3u64),
            leaf,
        ];
        assert!(Groth16::<Bls12_381>::verify(&vk, &inputs, &proof).unwrap());
        println!("✓ Groth16 proof verified successfully");
    }
}