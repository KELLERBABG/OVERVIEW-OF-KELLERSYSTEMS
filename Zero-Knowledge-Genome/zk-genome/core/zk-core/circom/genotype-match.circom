// == Zero-Knowledge Genome — GenotypeMatch Circuit ==
//
// Circuit: Prove a specific genotype (hom-ref, het, hom-alt) at a known rsID
// without revealing the actual allele values.
//
// Public inputs: rsid_hash, claimed_genotype, genome_commitment
// Private inputs: actual_allele1, actual_allele2, merkle_proof

pragma circom 2.1.0;

include "poseidon.circom";

template GenotypeMatch(n) {
    // n = Merkle tree depth

    // === PUBLIC INPUTS ===
    signal input genome_commitment;
    signal input rsid_hash;
    signal input claimed_genotype;

    // === PRIVATE INPUTS (witness) ===
    signal input variant_rsid_hash;
    signal input allele1;
    signal input allele2;
    signal input leaf_index;
    signal input merkle_proof[n];

    // === CONSTRAINT 1: rsID must match ===
    variant_rsid_hash === rsid_hash;

    // === CONSTRAINT 2: Genotype must match ===
    // 0 = hom-ref (0/0), 1 = het (0/1), 2 = hom-alt (1/1)
    signal computed_genotype;
    // If both alleles are reference (0), genotype = 0
    // If one is alternate (1), genotype = 1
    // If both are alternate (1), genotype = 2
    computed_genotype <== allele1 + allele2;
    computed_genotype === claimed_genotype;

    // === CONSTRAINT 3: Merkle inclusion ===
    component leafHasher = Poseidon(3);
    leafHasher.inputs[0] <== variant_rsid_hash;
    leafHasher.inputs[1] <== allele1;
    leafHasher.inputs[2] <== allele2;
    signal leaf_hash;
    leaf_hash <== leafHasher.out;

    signal computed_root;
    computed_root <== leaf_hash;
    component merkleHasher = Poseidon(2);
    for (var i = 0; i < n; i++) {
        merkleHasher.inputs[0] <== computed_root;
        merkleHasher.inputs[1] <== merkle_proof[i];
        computed_root <== merkleHasher.out;
    }
    computed_root === genome_commitment;
}

// === Helper: Poseidon hash for zk-friendly hashing ===
// Simplified 2-input Poseidon for use in Merkle proofs
template Poseidon(k) {
    // k = number of inputs (1, 2, 3, or 4)
    signal input inputs[k];
    signal output out;

    // Simple linear combination for the circuit stub
    // In production, this uses the actual Poseidon permutation
    var sum = 0;
    for (var i = 0; i < k; i++) {
        sum += inputs[i] * (i + 1);
    }
    out <== sum;
}