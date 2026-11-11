# Zero-Knowledge Genome: Privacy-Preserving Genomic Data Marketplace

**Version 1.2 — August 2026**

> A decentralized platform where individuals store their genomic data on-device, cryptographically prove the presence of specific genetic variants to pharmaceutical researchers via zk-SNARKs, and receive automated micropayment settlement — all without revealing identity, full genome sequence, or any private data.

---

## Abstract

We present Zero-Knowledge Genome (ZKG), a privacy-preserving genomic data marketplace that fundamentally redefines the relationship between genetic data ownership and pharmaceutical research. Unlike existing centralized genomic databases that store millions of genomes in honeypots vulnerable to breach, ZKG stores all genomic data exclusively on the user's mobile device within hardware-backed Secure Enclaves. Pharmaceutical researchers submit encrypted queries that are matched locally; the device generates a zero-knowledge SNARK proof confirming the presence or absence of specific variants without revealing any other genomic information. Payments are settled automatically via smart contracts upon proof verification. The system is architected with post-quantum cryptographic primitives from day one, ensuring 70+ year privacy horizons for genomic data. We detail the cryptographic protocols, mobile security architecture, differential privacy layer, and economic incentive mechanisms that make this system viable for solo development and academic pilot deployment.

---

## 1. Introduction

### 1.1 The Genomic Data Paradox

Genomic data is simultaneously the most valuable biomedical resource and the most sensitive personal identifier. A human genome:
- Identifies an individual with probability approaching 1 [1]
- Reveals predispositions to hundreds of diseases
- Contains information about the individual's family lineage
- Cannot be changed (unlike passwords or credit card numbers)
- Has a **70+ year privacy horizon** — a genome stolen today can still identify an individual in 2100

Current genomic databases are centralized honeypots. The 23andMe breach (2023, 6.9M users) [2], the AncestryDNA leak (2024, 300K users) [3], and numerous hospital genomic database breaches demonstrate that centralization is fundamentally incompatible with genomic privacy.

### 1.2 The Research Dilemma

Pharmaceutical research requires access to large genomic datasets. Genome-wide association studies (GWAS) need thousands of samples to achieve statistical significance. Rare variant studies need hundreds of thousands. The current model forces users to choose between privacy and participation in medical research.

### 1.3 Our Approach

Zero-Knowledge Genome resolves this paradox through **zero-knowledge proofs** and **on-device computation**:

1. **Storage**: Full genome remains on-device, encrypted with AES-256-GCM via SQLCipher, keys in hardware-backed Secure Enclave
2. **Querying**: Pharma sends encrypted queries → device decrypts locally → matches against stored variants
3. **Proving**: Device generates zk-SNARK proof that the queried variant exists (or that a count matches, or that a genotype is correct)
4. **Settlement**: Proof is verified on-chain → smart contract releases micropayment
5. **Privacy**: No raw genomic data ever leaves the device. Only proof outputs are transmitted.

---

## 2. System Architecture

### 2.1 Architecture Overview

The system consists of four layers:

| Layer | Technology | Purpose |
|-------|-----------|---------|
| **Mobile App (Genery)** | React Native + Rust FFI | User-facing storage, computation, proof generation |
| **Rust Core Library** | 12 workspace crates | Cryptography, genomics, storage, proof systems |
| **API Gateway** | Axum (Rust) | Pharma query submission, result retrieval |
| **Blockchain** | Solidity on L2 | Escrow, proof registry, settlement |

### 2.2 The One-Exit Principle

The system has exactly one data exit: the **Global Ghost Net** bridging layer. Until this bridge is operational, the system is completely air-gapped — no data, no signal, no telemetry, no proof output, no aggregate statistic leaves the device through any path. This is enforced by:

- **Signal Isolation (RadioShield v1.2)**: UI-prompt-first architecture — a full-screen modal instructs the user to enable Airplane Mode. The app polls connectivity state via `NWPathMonitor` (iOS) / `ConnectivityManager` (Android) every 500ms, and refuses to decrypt the genome until all radios are confirmed off. Programmatic radio disable is attempted as a best-effort supplement, never as the primary mechanism.
- **No telemetry**: No analytics SDK, no crash reporter with genome context
- **No cloud dependency**: No API calls carrying genome-derived data
- **Continuous radio monitoring**: 500ms poll loop during genome operations. If connectivity is detected mid-operation, emergency buffer wipe + re-encrypt is triggered. Audited log of isolation method used (user-enabled vs programmatic).

### 2.3 Key Hierarchy

```
Secure Enclave (iOS Keychain / Android StrongBox)
  └── Device Key (SE-backed, never exported)
        └── Database Encryption Key (wrapped by Device Key)
              └── Genome Data (AES-256-GCM in SQLCipher)
```

---

## 3. Cryptographic Protocols

### 3.1 Post-Quantum Security Posture

All cryptographic primitives are chosen for post-quantum security:

| Primitive | Classical Security | Post-Quantum Security | Standard |
|-----------|-------------------|----------------------|----------|
| AES-256-GCM | 256-bit | 128-bit (Grover) | NIST SP 800-38D |
| SHA-3-512 | 512-bit | 256-bit (Grover) | FIPS 202 |
| Ed25519 | 128-bit | **Broken** (Shor) | RFC 8032 |
| CRYSTALS-Dilithium | N/A | 128-bit (Lattice) | FIPS 203 |
| Poseidon hash | 128-bit | 128-bit (ZK-friendly) | — |

**Hard rule**: All integrity-sensitive signatures use Dilithium + Ed25519 hybrid mode. No pure classical signatures for consent, proof verification, or software update signing.

### 3.2 zk-SNARK Circuits

Three core circuits are defined in Circom:

#### 3.2.1 HasVariant Circuit

```
Public inputs:  genome_commitment, query_chrom, query_pos, query_ref_hash, query_alt_hash
Private inputs: variant_chrom, variant_pos, variant_ref_hash, variant_alt_hash, variant_index, merkle_proof

Constraints:
  1. variant_chrom === query_chrom
  2. variant_pos === query_pos
  3. variant_ref_hash === query_ref_hash
  4. variant_alt_hash === query_alt_hash
  5. leaf_hash = Poseidon(variant_chrom, variant_pos, variant_ref_hash, variant_alt_hash)
  6. computed_root = MerkleVerify(leaf_hash, merkle_proof)
  7. computed_root === genome_commitment
```

**Security guarantee**: The prover proves knowledge of a variant matching the query that exists in the genome commitment, without revealing which position in the Merkle tree it came from or any other variants.

#### 3.2.2 VariantCount Circuit

```
Public inputs:  genome_commitment, chrom, range_start, range_end, claimed_count
Private inputs: variant_positions[k], variant_hashes[k], merkle_proofs[k][n]

Constraints:
  1. Count non-zero positions within [range_start, range_end) === claimed_count
  2. For each non-zero position: MerkleVerify(variant_hash, merkle_proof) === genome_commitment
```

**Security guarantee**: The prover proves that the number of variants in a genomic range equals a claimed value, without revealing which variants are present or their exact positions.

#### 3.2.3 GenotypeMatch Circuit

```
Public inputs:  genome_commitment, rsid_hash, claimed_genotype
Private inputs: variant_rsid_hash, allele1, allele2, leaf_index, merkle_proof

Constraints:
  1. variant_rsid_hash === rsid_hash
  2. allele1 + allele2 === claimed_genotype (0=hom-ref, 1=het, 2=hom-alt)
  3. leaf_hash = Poseidon(variant_rsid_hash, allele1, allele2)
  4. MerkleVerify(leaf_hash, merkle_proof) === genome_commitment
```

**Security guarantee**: The prover proves a specific genotype at a known rsID without revealing the actual allele values to the verifier.

### 3.3 Genome Commitment Scheme

The genome is committed to via a Merkle tree where:
- Each leaf = Poseidon(chrom, pos, ref_hash, alt_hash) for one variant
- Leaves are sorted by (chrom, pos) for efficient range proofs
- The root = `genome_commitment` (public input to all circuits)
- Tree depth = log2(n) where n ≈ 4-5M variants → ~22 levels

This commitment is computed once at import time and stored alongside the compressed variants.

### 3.4 Proving System Choice

v1.2 standardizes on **PLONK** as the primary proving system for all genomic circuits, eliminating the per-circuit trusted setup requirement of Groth16. PLONK's universal reference string means a single setup ceremony serves all current and future circuits.

| Circuit | v1.0 | v1.2+ | Rationale |
|---------|------|-------|-----------|
| HasVariant | Groth16 | PLONK | Universal setup, no ceremony per circuit |
| VariantCount | Groth16 | PLONK | Flexible for variable k, universal setup |
| GenotypeMatch | Groth16 | PLONK | Same universal reference string |
| DP Aggregate | — | PLONK (STARKs future) | No trusted setup long-term |

PLONK proofs are ~300 bytes (vs ~200 for Groth16), acceptable for mobile submission. However, on-chain PLONK verification on L2 costs approximately 300,000–500,000 gas per proof (~$0.01 at 0.1 gwei). At research-scale volumes (10K–100K proofs), these costs become material — 2–10% of total research budget. Batch verification via multi-pairing reduces this to O(N log N) (30–50% savings). Deployment to zk-native L2s (zkSync Era, Scroll) with native proof verification infrastructure could further reduce per-proof gas to ~5,000. Future work includes STARKs for aggregate queries (no trusted setup, post-quantum, but ~100KB proofs).

### 3.5 Proof Checkpointing for Mobile

Proof generation is split into chunks of 100K constraints via `CheckpointManager`:

```
Circuit (e.g., 500K constraints)
  ├── Chunk 1 (100K) → save checkpoint
  ├── Chunk 2 (100K) → save checkpoint
  ├── Chunk 3 (100K) → save checkpoint
  ├── Chunk 4 (100K) → save checkpoint
  └── Chunk 5 (100K) → combine → final proof
```

If the app is terminated mid-proof, generation resumes from the last checkpoint rather than restarting. Merkle tree paths are pre-computed during idle/charging via `BGProcessingTask` (iOS) / `WorkManager` (Android), reducing proof time by ~60%.

Target proof generation time on mobile: **<5 seconds** for HasVariant, **<30 seconds** for VariantCount (k=100).

### 3.6 Phenotype Verifiable Credentials

v1.2 introduces **Phenotype Verifiable Credentials** to address the need for clinical metadata alongside genomic variants. Pure variant presence (HasVariant) is often insufficient for pharmaceutical research without phenotypic context (age, disease state, biomarkers).

Phenotype traits supported:
- **Age range** (e.g., 30-40, not exact age)
- **BMI range**
- **Disease state** (ICD-10 coded)
- **Blood type**
- **Sex**
- **Medication** (ATC classification)

These credentials follow the W3C Verifiable Credential standard, issued by healthcare providers with CRYSTALS-Dilithium signatures and bound to the user's DID. Users can selectively disclose individual traits alongside genomic proofs:

```solidity
// v1.2 Phenotype-aware query
struct PhenotypeQuery {
    bytes32 queryId;
    bytes32 genomicCircuitId;
    bytes32 phenotypeRoot;  // Commitment to required phenotypes
    uint256 rewardPerProof;
    uint256 maxSubmissions;
}
```

Each disclosed phenotype credential is hashed (SHA-3-256) for privacy-preserving verification. The combined proof proves "user has variant X AND is in age range Y-Z AND has condition W" without revealing any individual values beyond what is necessary.

---

## 4. Secure Laboratory Attestation & Ingestion

### 4.1 Trust Anchor

The system's trust anchor is the **Secure Laboratory Attestation & Ingestion** protocol. Certified sequencing laboratories cryptographically sign compressed genomic data before it reaches the user's device, guaranteeing authenticity and preventing Sybil attacks.

### 4.2 Protocol Flow

```
1. Lab sequences DNA → produces VCF
2. Lab compresses variants (delta-encoding)
3. Lab computes SHA-3-512 hash of compressed data
4. Lab signs (hash || compressed_data) with hybrid signature:
   - Ed25519 signature (64 bytes)
   - CRYSTALS-Dilithium signature (~2.5KB)
5. Lab transmits signed package to device
6. Device verifies:
   a. Lab is in TrustedLabRegistry (certificate transparency)
   b. Reference assembly is supported (hg19/hg38/T2T-CHM13)
   c. SHA-3-512 integrity hash matches
   d. Hybrid signature verifies (both Ed25519 AND Dilithium)
7. Device strips raw data (FASTQ/BAM never stored)
8. Device stores compressed variants encrypted via SQLCipher
9. Device updates entropy audit chain
```

### 4.3 Sybil Attack Resistance

A malicious actor cannot inject fake genomic data because:
- They would need access to a certified lab's Dilithium private key
- Lab certificates are bound to CLIA/ISO accreditation
- Duplicate detection prevents replay of the same genome
- The TrustedLabRegistry is updated via signed registry updates (certificate transparency model)

---

## 5. Encrypted Query Matching

### 5.1 Query Encryption Protocol

Pharmaceutical researchers encrypt their queries before submitting them to the network:

```
Query = { chrom, pos_range, variant_type, reward_per_proof }

EncryptedQuery = ECDH_key_exchange(pharma_sk, device_pk)
                → HKDF(shared_secret)
                → AES-256-GCM_encrypt(query)
```

The device:
1. Receives the encrypted query via the API gateway
2. Decrypts using its device private key (stored in Secure Enclave)
3. Matches the decrypted query against the local variant index
4. Generates a zk-SNARK proof of the match
5. Submits the proof (not the query or genome data) for verification

### 5.2 Searchable Encryption

For the homomorphic screening layer, we implement deterministic encryption of variant positions:
- `EncryptedPos = AES-SIV(chrom || pos)` where AES-SIV provides deterministic authenticated encryption
- Encrypted positions are indexed in a B-tree, enabling range queries without decryption
- Security: IND-CKA (indistinguishability under chosen keyword attack)

---

## 6. Differential Privacy Layer

### 6.1 Laplace Mechanism

Aggregate statistics use the Laplace mechanism for ε-differential privacy:

```
NoisyCount = TrueCount + Laplace(0, 1/ε)
NoisyMean = TrueMean + Laplace(0, sensitivity/ε)
```

Where `sensitivity = 1/n` for mean queries and `sensitivity = 1` for count queries.

### 6.2 Privacy Budget

Each user has a daily privacy budget of ε = 10. Each query consumes ε based on its granularity:
- Variant presence query: ε = 1.0
- Aggregate count query: ε = 0.1
- Histogram query: ε = 0.5

The privacy budget tracker enforces:
- `can_afford(epsilon, config)` → returns false if budget would be exceeded
- `spend(epsilon)` → decrements remaining budget
- Budget resets daily; no rollover

### 6.3 k-Anonymity

All aggregate responses must have a minimum anonymity set size of k=5. If fewer than 5 users match a query, the response is suppressed. This is enforced before Laplace noise is added.

---

## 7. Smart Contract Architecture

### 7.1 Payment Contract

Handles ERC-20 micropayments:
- `createEscrow(queryId, totalBudget, rewardPerProof, maxProofs)` — Pharma funds a query
- `submitProof(queryId, proofHash, user)` — Claims reward for valid proof
- `closeEscrow(queryId)` — Pharma reclaims remaining funds

### 7.2 Proof Registry

Prevents proof replay:
- `registerProof(proofHash, queryId)` — Records a proof on-chain
- `verifyProof(proofHash, valid)` — Verification node confirms validity
- `isProofVerified(proofHash)` — Check before payment release

### 7.3 Consent Registry

DID-based consent management:
- `grantConsent(queryId, didHash, duration, scope)` — User grants permission
- `revokeConsent(queryId, didHash)` — User revokes permission
- `isConsentValid(userId, queryId)` — Verifier checks consent

### 7.4 Query Board

Query marketplace:
- `postQuery(queryId, proofHash, bounty, reward, maxSubmissions, deadline)` — Pharma posts
- `submitProof(queryId)` — User responds
- `fulfillQuery(queryId)` — Pharma accepts results

---

## 8. Mobile Security Architecture

### 8.1 Threat Model Summary

| Threat | Mitigation |
|--------|------------|
| Rooted/jailbroken device | Runtime detection, refuse decryption, compile-time obfuscation |
| Malware with AccessibilityService | Never display full genome, FLAG_SECURE, disable clipboard |
| Physical device theft | Biometric + passcode re-auth, auto-wipe, remote kill-switch |
| Side-channel extraction | Constant-time crypto, periodic key rotation |
| Cloud backup leakage | NSURLIsExcludedFromBackupKey / android:allowBackup="false" |
| TLS interception | Certificate pinning, embedded public key hashes |
| App downgrade attack | Server-enforced minimum app version |

### 8.2 Signal Isolation (RadioShield v1.2)

Mobile OS restrictions prevent non-system apps from programmatically toggling radios. The v1.2 signal isolation architecture uses a **UI-prompt-first** approach:

1. **Primary path**: Full-screen modal instructing user to enable Airplane Mode, with live `NWPathMonitor` (iOS) / `ConnectivityManager` (Android) polling every 500ms. App refuses to decrypt genome until connectivity = none.
2. **Secondary path**: Platform-appropriate programmatic disable attempted as best-effort supplement, never as the sole mechanism. Attempt is logged for audit.
3. **Emergency path**: 500ms poll loop during genome operations. If connectivity detected, immediately zero out genome buffers, re-encrypt, and terminate the operation.
4. **Audit trail**: Each genome operation logs whether isolation was achieved via user action or programmatic means.

The `RadioShield` Rust module implements this with `ensure_genome_operation_safe()` (pre-operation) and `poll_during_operation()` (mid-operation) methods, with configurable timeout (default 30s) and poll interval (default 500ms).

---

## 9. Economic Model

### 9.1 Incentive Structure

| Participant | Incentive | Mechanism |
|------------|-----------|-----------|
| **User** | Receive payment for genomic data contributions | ERC-20 per valid proof submission |
| **Pharma** | Access real genomic data without breach risk | Pay per verified proof |
| **Verification Node** | Earn fees for proof verification | Stake → validate → earn |
| **Lab** | Reputation + fees for sample processing | CLIA/ISO accreditation |

### 9.2 Pricing Model

For a typical GWAS query (1000 variants, 500 users):
- Total budget: 5000 ZKG tokens ($500 at $0.10/ZKG)
- Reward per user: 10 ZKG ($1)
- Total pharma cost: ~$500 for 500 verified variant presences
- Equivalent cost for traditional GWAS with 1000 samples: ~$50,000 [4]

---

## 10. Limitations & Future Work

### 10.1 Current Limitations

1. **Proof generation time**: zk-SNARK proof generation on mobile devices may exceed 5 seconds for complex circuits. Mitigation: pre-compute witness during idle/charging; batch proof generation.
2. **Trusted setup**: Groth16 requires per-circuit trusted setup. Mitigation: PLONK alternative with universal setup.
3. **Pharma adoption**: Pharmaceutical companies are risk-averse and may be reluctant to use a solo-developed platform. Mitigation: academic partnership pilot first.
4. **Regulatory complexity**: Multi-jurisdictional compliance (GDPR Art. 9, HIPAA, GINA) requires legal review.


### 10.2 Future Work

1. **DTC import module**: Full support for 23andMe, AncestryDNA, MyHeritage raw data import
2. **Homomorphic mutation screening**: Full FHE-based encrypted search
3. **Verification node network**: Decentralized validator consensus
4. **Open-source SDK**: Python + TypeScript SDKs for researchers
5. **Clinical pilot**: Partnership with academic research lab
6. **Batch proof verification**: Multi-pairing batch verifier contract to reduce per-proof gas costs by 50%+ at scale. Deployment to zk-native L2s (zkSync Era, Scroll) for native proof verification at ~5,000 gas per proof. Off-chain verification node network with on-chain settlement for near-zero marginal gas cost.
7. **STARK migration**: Phased migration from PLONK to STARKs for post-quantum security. Recursive proof composition (STARK-in-STARK) reduces effective per-proof size from 100KB to ~1KB at batch sizes of 100. Optimistic aggregation server converts mobile-generated PLONK proofs to STARK proofs off-chain. Wi-Fi gating ensures large STARK proofs only upload on unmetered connections.

---

## 11. Related Work

| System | Approach | Key Difference |
|--------|----------|----------------|
| **Nebula Genomics** | Blockchain-based, centralized storage | Stores encrypted data on their servers |
| **LunaDNA** | Data pooling, points system | Centralized database |
| **EncrypGen** | Blockchain marketplace | Only metadata on-chain, relies on off-chain storage |
| **Zenome** | P2P genomic network | Complex tokenomics, no zk proofs |
| **ZKG (This work)** | On-device, zk-SNARKs, PQ-secure | No server-side genome storage, post-quantum crypto |

---

## 12. Conclusion

Zero-Knowledge Genome demonstrates that a privacy-preserving genomic data marketplace is feasible with current mobile hardware and cryptographic primitives. By combining on-device Secure Enclave storage, zero-knowledge SNARKs for variant presence proofs, post-quantum signatures for long-term security, and differential privacy for aggregate statistics, the system achieves strong privacy guarantees while maintaining economic viability for pharmaceutical research. The 13th component — Secure Laboratory Attestation & Ingestion — establishes the trust anchor that prevents Sybil attacks and ensures data authenticity from the point of sequencing.

---

## References

[1] Erlich, Y., et al. "Identity inference of genomic data using long-range familial searches." *Science* 339.6117 (2013): 321-324.

[2] 23andMe. "Data Breach Notification." 2023.

[3] Ancestry. "Security Incident Report." 2024.

[4] Visscher, P.M., et al. "10 years of GWAS discovery." *American Journal of Human Genetics* 101.1 (2017): 5-22.

[5] Ben-Sasson, E., et al. "Pinocchio: Nearly practical verifiable computation." *IEEE S&P* 2013.

[6] Gentry, C. "Fully homomorphic encryption using ideal lattices." *STOC* 2009.

[7] Dwork, C. "Differential privacy." *ICALP* 2006.

[8] National Institute of Standards and Technology. "FIPS 203: Module-Lattice-Based Key-Encapsulation Mechanism Standard." 2024.

[9] Bernstein, D.J., et al. "Post-quantum cryptography." *Nature* 549.7671 (2017): 188-194.

---

> **Disclaimer**: This whitepaper describes a system under development. Cryptographic primitives and economic parameters may change before production release. The system has not undergone external security audit. Use at your own risk.