// == Zero-Knowledge Genome — VariantCount Circuit ==
//
// Circuit: Prove that the number of variants in a genomic range [start, end)
// equals a claimed count, without revealing which variants or their positions.
//
// Public inputs: genome_commitment, chrom, start, end, claimed_count
// Private inputs: variant_positions[], merkle_proofs[]

pragma circom 2.1.0;

include "poseidon.circom";

template VariantCount(k, n) {
    // k = maximum number of variants expected in range
    // n = Merkle tree depth

    // === PUBLIC INPUTS ===
    signal input genome_commitment;
    signal input chrom;
    signal input range_start;
    signal input range_end;
    signal input claimed_count;

    // === PRIVATE INPUTS (witness) ===
    // Up to k variant positions in the range
    signal input variant_positions[k];
    signal input variant_hashes[k];

    // Merkle inclusion proofs for each variant
    signal input merkle_proofs[k][n];

    // === CONSTRAINT 1: Count correctness ===
    // The claimed count must equal the number of non-zero positions provided
    // We use a selector technique: each valid position contributes 1
    signal total_count;
    var count = 0;

    // Count using range constraints (position > 0 means variant exists)
    for (var i = 0; i < k; i++) {
        // is_valid = 1 if variant_positions[i] > 0 AND in range, else 0
        signal is_in_range;
        is_in_range <== (variant_positions[i] >= range_start) * (variant_positions[i] < range_end);

        // Bitwise: if position is non-zero AND in range, count it
        signal non_zero;
        non_zero <== variant_positions[i] > 0 ? 1 : 0;
        count += non_zero * is_in_range;
    }
    claimed_count === count;

    // === CONSTRAINT 2: Merkle inclusion for each variant ===
    // Verify each variant exists in the genome commitment
    for (var i = 0; i < k; i++) {
        // Skip verification for empty slots (position = 0)
        signal skip;
        skip <== variant_positions[i] == 0 ? 1 : 0;

        // Compute leaf hash
        component leafHasher = Poseidon(1);
        leafHasher.inputs[0] <== variant_hashes[i];
        signal computed;
        computed <== leafHasher.out;

        // Follow Merkle path
        component merkleHasher = Poseidon(2);
        for (var j = 0; j < n; j++) {
            merkleHasher.inputs[0] <== computed;
            merkleHasher.inputs[1] <== merkle_proofs[i][j];
            computed <== merkleHasher.out;
        }

        // If not skipped, computed must equal commitment
        // (skip * something + (1-skip) * (computed - commitment) == 0)
        signal constraint;
        constraint <== (1 - skip) * (computed - genome_commitment);
        constraint === 0;
    }
}