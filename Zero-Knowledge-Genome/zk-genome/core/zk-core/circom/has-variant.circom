// == Zero-Knowledge Genome — HasVariant Circuit ==
//
// Circuit: Prove knowledge of a variant at (chrom, pos) with ref/alt alleles
// without revealing the genome.
//
// Public inputs:  merkle_root (commitment to the genome)
// Private inputs: chrom, pos, ref_hash, alt_hash, merkle_proof
//
// The prover knows the genome and wants to prove that a specific variant
// exists without revealing which part of the genome it came from.

pragma circom 2.1.0;

include "poseidon.circom";

template HasVariant(n) {
    // n = number of variants in the genome (public parameter)

    // === PUBLIC INPUTS ===
    // A commitment to the entire genome (Poseidon hash of all variant hashes)
    signal input genome_commitment;

    // The query: what variant are we looking for?
    signal input query_chrom;
    signal input query_pos;
    signal input query_ref_hash;
    signal input query_alt_hash;

    // === PRIVATE INPUTS (witness — known only to prover) ===
    // The specific variant's position in the Merkle tree
    signal input variant_index;

    // The variant data at that position
    signal input variant_chrom;
    signal input variant_pos;
    signal input variant_ref_hash;
    signal input variant_alt_hash;

    // Merkle proof (path elements for inclusion)
    signal input merkle_proof[n];

    // === CONSTRAINTS ===

    // 1. The variant must match the query
    variant_chrom === query_chrom;
    variant_pos   === query_pos;
    variant_ref_hash === query_ref_hash;
    variant_alt_hash === query_alt_hash;

    // 2. Compute the leaf hash = Poseidon(variant_chrom, variant_pos, variant_ref_hash, variant_alt_hash)
    component leafHasher = Poseidon(4);
    leafHasher.inputs[0] <== variant_chrom;
    leafHasher.inputs[1] <== variant_pos;
    leafHasher.inputs[2] <== variant_ref_hash;
    leafHasher.inputs[3] <== variant_alt_hash;
    signal leaf_hash;
    leaf_hash <== leafHasher.out;

    // 3. Verify Merkle inclusion: recompute root from leaf + proof path
    signal computed_root;
    computed_root <== leaf_hash;

    // Merkle tree verification: at each level, hash the pair
    // Uses a binary Merkle tree of depth log2(n)
    component merkleHasher = Poseidon(2);
    for (var i = 0; i < n; i++) {
        merkleHasher.inputs[0] <== computed_root;
        merkleHasher.inputs[1] <== merkle_proof[i];
        computed_root <== merkleHasher.out;
    }

    // 4. The computed root must match the public genome commitment
    computed_root === genome_commitment;

    // === OUTPUT ===
    // If all constraints pass, the proof is valid
    // No explicit output needed — constraints enforce correctness
}