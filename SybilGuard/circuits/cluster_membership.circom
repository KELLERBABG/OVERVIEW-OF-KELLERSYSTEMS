// Cluster Membership Proof Circuit
// 
// Proves: "I am a member of a legitimate trust cluster of size >= MIN_SIZE"
// Without revealing: which cluster, which node, or any neighbor relationships.
//
// Public inputs:
//   - cluster_nullifier (hash of cluster seed)
//   - root_merkle (Merkle root of the cluster's public keys)
//   - min_size (minimum cluster size requirement)
//
// Private inputs:
//   - my_secret_key (ed25519 secret key material)
//   - my_public_key (ed25519 public key, as field element)
//   - merkle_proof (Merkle proof of inclusion in cluster)
//   - cluster_size (actual cluster size)
//
// This is a simplified educational circuit. Full production circuit would
// use Poseidon hash for Merkle trees and ed25519 curve points.

pragma circom 2.1.0;

// ---- Helper: Check two values are equal ----
template IsEqual() {
    signal input a;
    signal input b;
    signal output out;
    out <== a == b ? 1 : 0;
}

// ---- Helper: Range check (value < bound) ----
template LessThan(bits) {
    assert(bits <= 252);
    signal input in[2];
    signal output out;

    component comp = Num2Bits(bits + 1);
    comp.in <== in[0] + (1 << bits) - in[1];
    out <== 1 - comp.out[bits];
}

// ---- Helper: Numeric to bits ----
template Num2Bits(n) {
    signal input in;
    signal output out[n];
    var lc = 0;
    for (var i = 0; i < n; i++) {
        out[i] <-- (in >> i) & 1;
        out[i] * (out[i] - 1) === 0;
        lc += (2 ** i) * out[i];
    }
    lc === in;
}

// ---- Poseidon hash (simplified with MiMC for demo) ----
// In production: use circomlib Poseidon
template MimcHash() {
    signal input in;
    signal output out;
    
    // Simplified MiMC hash: out = (in + key)^3 mod p
    // This is a toy construction for demonstration only
    var key = 123456789;
    var p = 21888242871839275222246405745257275088548364400416034343698204186575808495617;
    
    signal x;
    x <== in + key;
    out <== (x * x * x) % p;
}

// ---- Merkle proof verification (simplified, depth=3) ----
template MerkleProof(depth) {
    signal input leaf;
    signal input root;
    signal input siblings[depth];
    signal input directions[depth];  // 0 = left, 1 = right
    
    signal output valid;
    
    component hashers[depth];
    component hash = MimcHash();
    
    signal current;
    current <== leaf;
    
    for (var i = 0; i < depth; i++) {
        hashers[i] = MimcHash();
        
        // If direction is 0: current || sibling
        // If direction is 1: sibling || current
        signal left;
        signal right;
        left <== (1 - directions[i]) * current + directions[i] * siblings[i];
        right <== (1 - directions[i]) * siblings[i] + directions[i] * current;
        
        // Simplified: hash(left + right) using MiMC-like construction
        hashers[i].in <== left + right;
        current <== hashers[i].out;
    }
    
    // Verify computed root matches provided root
    component eq = IsEqual();
    eq.a <== current;
    eq.b <== root;
    valid <== eq.out;
}

// ---- Main Cluster Membership Circuit ----
template ClusterMembership(min_depth) {
    // Public inputs
    signal input cluster_nullifier;     // Hash identifying the cluster
    signal input root_merkle;           // Merkle root of cluster members
    signal input min_size;              // Minimum size requirement
    
    // Private inputs (witness)
    signal input my_secret;             // My secret seed
    signal input my_public;             // My public key (as field element)
    signal input merkle_siblings[min_depth]; // Merkle proof siblings
    signal input merkle_directions[min_depth]; // Merkle proof directions
    signal input actual_cluster_size;   // Actual size of my cluster
    
    signal output valid;                // 1 if proof is valid
    
    // 1. Compute my nullifier: nullifier = Hash(secret)
    component hash = MimcHash();
    hash.in <== my_secret;
    signal my_nullifier;
    my_nullifier <== hash.out;
    
    // 2. Verify my public key is derived from my secret (simplified)
    // In production: verify my_public = ed25519_base_point * my_secret
    component key_hash = MimcHash();
    key_hash.in <== my_secret;
    signal derived_public;
    derived_public <== key_hash.out;  // simplified! not real ed25519
    
    component key_eq = IsEqual();
    key_eq.a <== derived_public;
    key_eq.b <== my_public;
    signal valid_key;
    valid_key <== key_eq.out;
    
    // 3. Verify Merkle proof of inclusion in the cluster
    component merkle = MerkleProof(min_depth);
    merkle.leaf <== derived_public;
    merkle.root <== root_merkle;
    for (var i = 0; i < min_depth; i++) {
        merkle.siblings[i] <== merkle_siblings[i];
        merkle.directions[i] <== merkle_directions[i];
    }
    signal valid_merkle;
    valid_merkle <== merkle.valid;
    
    // 4. Verify cluster size >= min_size
    component size_check = LessThan(32);
    size_check.in[0] <== actual_cluster_size;
    size_check.in[1] <== min_size - 1;  // actual >= min means actual > min-1
    signal valid_size;
    valid_size <== 1 - size_check.out;  // invert: actual > min-1 means actual >= min
    
    // 5. Verify nullifier matches the cluster nullifier (simplified)
    component null_hash = MimcHash();
    null_hash.in <== my_nullifier;
    signal computed_cluster_nullifier;
    computed_cluster_nullifier <== null_hash.out;
    
    component null_eq = IsEqual();
    null_eq.a <== computed_cluster_nullifier;
    null_eq.b <== cluster_nullifier;
    signal valid_nullifier;
    valid_nullifier <== null_eq.out;
    
    // All conditions must be satisfied
    valid <== valid_key * valid_merkle * valid_size * valid_nullifier;
}

// Instantiate with depth 5 (supports up to 2^5 = 32 members per cluster)
component main = ClusterMembership(5);