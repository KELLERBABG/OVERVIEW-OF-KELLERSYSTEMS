# Phase 3 — Integrity & Privacy (Weeks 10–13)

**Goal:** Nodes produce zero-knowledge proofs that their computation was correct (zk-SNARKs), and sensitive data can be processed under homomorphic encryption. This enables trustless verification and data privacy.

---

## 1. Packages Involved

| Package | Responsibility |
|---------|---------------|
| `@fungal/zk-integrity` | Circom circuit design, proof generation (prover), proof verification (verifier), batch aggregation |
| `@fungal/homomorphic` | Integration with TFHE-rs or SEAL compiled to Wasm, key management, encrypted execution |

---

## 2. zk-SNARK Integrity (`packages/zk-integrity`)

### 2.1 Purpose
When a worker node claims it computed a result, a zk-SNARK proves that it actually ran the correct computation without the requester needing to re-execute. This is the "verifiable computation" layer.

### 2.2 Architecture

```
┌────────────────────────────────────────────────────────────┐
│                    Task Requester                          │
│  1. Generate proving key & verification key (one-time)     │
│  2. Send proving key + task to worker                      │
│  3. Receive result + proof                                 │
│  4. Verify proof with verification key                     │
└────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌────────────────────────────────────────────────────────────┐
│                     Worker Node                            │
│  1. Receive task + proving key                             │
│  2. Execute computation                                    │
│  3. Generate execution trace                               │
│  4. Create zk-SNARK proof (prover)                         │
│  5. Return result + proof                                  │
└────────────────────────────────────────────────────────────┘
```

### 2.3 File Structure

```
packages/zk-integrity/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── prover.ts            # zk-SNARK proof generation (worker-side)
│   ├── verifier.ts          # Proof verification (requester-side)
│   ├── circuits/
│   │   ├── compute-wasm.ts  # Generic Wasm computation circuit (in TS)
│   │   ├── arithmetic.ts    # Specific: arithmetic circuit
│   │   └── aggregation.ts   # Batch proof aggregation circuit
│   ├── key-manager.ts       # Proving/verification key storage
│   ├── types.ts
│   └── config.ts
├── circuits/                 # Circom circuits (compiled to .wasm + .zkey)
│   ├── compute.circom
│   ├── arithmetic.circom
│   └── aggregation.circom
├── tests/
│   ├── prover.test.ts
│   ├── verifier.test.ts
│   └── integration.test.ts
└── scripts/
    ├── setup.sh              # Download powers of tau, compile circuits
    └── download-keys.sh      # Download proving keys for known circuits
```

### 2.4 Technology Choice

For browser compatibility, use **snarkjs** (pure JS, runs in browser):

```bash
pnpm add snarkjs
pnpm add -D @types/snarkjs
```

For circuit design, use **Circom** (compiled separately, not in browser):

```bash
# Install Circom (system-level, not in pnpm)
curl -L https://github.com/iden3/circom/releases/download/v2.1.6/circom-linux-amd64 -o /usr/local/bin/circom
chmod +x /usr/local/bin/circom
```

### 2.5 Circom Circuit — Generic Computation Verifier

```circom
// circuits/compute.circom
// This circuit verifies that a given Wasm computation was executed correctly.
//
// In practice, we use a "commit-and-prove" approach:
//   1. Worker commits to (input, output, executionTrace)
//   2. Circuit verifies that output = execute(input) using the execution trace
//
// For the MVP, we start with a simpler arithmetic circuit.

pragma circom 2.1.0;

template Arithmetic(n) {
    signal input a;
    signal input b;
    signal input op;  // 0=add, 1=sub, 2=mul, 3=div
    signal output c;

    component selectors[4];

    // Each selector checks if op matches and computes result
    selectors[0] = IsEqual();
    selectors[0].in[0] <== op;
    selectors[0].in[1] <== 0;
    selectors[1] = IsEqual();
    selectors[1].in[0] <== op;
    selectors[1].in[1] <== 1;
    selectors[2] = IsEqual();
    selectors[2].in[0] <== op;
    selectors[2].in[1] <== 2;
    selectors[3] = IsEqual();
    selectors[3].in[0] <== op;
    selectors[3].in[1] <== 3;

    var sum = 0;
    sum += selectors[0].out * (a + b);
    sum += selectors[1].out * (a - b);
    sum += selectors[2].out * (a * b);
    sum += selectors[3].out * (a \ b);

    c <== sum;
}

component main = Arithmetic(256);
```

### 2.6 Prover (Worker-Side)

```typescript
// src/prover.ts
import { groth16 } from 'snarkjs';

export interface ProofOptions {
  /** Path to compiled circuit .wasm */
  wasmPath: string;
  /** Path to proving key .zkey */
  zkeyPath: string;
}

export class SnarkProver {
  private wasmPath: string;
  private zkeyPath: string;

  constructor(options: ProofOptions) {
    this.wasmPath = options.wasmPath;
    this.zkeyPath = options.zkeyPath;
  }

  /**
   * Generate a zk-SNARK proof that a computation was performed correctly.
   *
   * @param publicInputs - Values both prover and verifier know (e.g., input hash, output hash)
   * @param privateInputs - Values only the prover knows (e.g., full input, trace)
   * @returns proof and public signals
   */
  async prove(
    publicInputs: Record<string, bigint | string>,
    privateInputs: Record<string, bigint | string>,
  ): Promise<{ proof: object; publicSignals: string[] }> {
    const fullInput = { ...publicInputs, ...privateInputs };

    // Generate proof using groth16
    const { proof, publicSignals } = await groth16.fullProve(
      fullInput,
      this.wasmPath,
      this.zkeyPath,
    );

    return { proof, publicSignals };
  }
}
```

### 2.7 Verifier (Requester-Side)

```typescript
// src/verifier.ts
import { groth16 } from 'snarkjs';

export class SnarkVerifier {
  private verificationKey: object;

  constructor(verificationKey: object) {
    this.verificationKey = verificationKey;
  }

  /**
   * Verify a zk-SNARK proof without re-executing the computation.
   *
   * @returns true if the proof is valid
   */
  async verify(
    proof: object,
    publicSignals: string[],
  ): Promise<boolean> {
    return groth16.verify(this.verificationKey, publicSignals, proof);
  }
}
```

### 2.8 Key Manager

```typescript
// src/key-manager.ts
export class KeyManager {
  private storage: Storage; // IndexedDB or localStorage

  constructor() {
    this.storage = typeof indexedDB !== 'undefined'
      ? new IndexedDBStorage('zk-keys')
      : new LocalStorageStorage('zk-keys');
  }

  async storeVerificationKey(circuitId: string, key: object): Promise<void> {
    await this.storage.set(`vk:${circuitId}`, key);
  }

  async getVerificationKey(circuitId: string): Promise<object | null> {
    return this.storage.get(`vk:${circuitId}`);
  }

  async storeProvingKey(circuitId: string, key: ArrayBuffer): Promise<void> {
    await this.storage.set(`pk:${circuitId}`, key);
  }

  async getProvingKey(circuitId: string): Promise<ArrayBuffer | null> {
    return this.storage.get(`pk:${circuitId}`);
  }

  /** Download keys from a trusted source (IPFS, CDN) */
  async downloadKeys(circuitId: string, urls: { vk: string; pk: string }): Promise<void> {
    const vkResp = await fetch(urls.vk);
    const vk = await vkResp.json();
    await this.storeVerificationKey(circuitId, vk);

    const pkResp = await fetch(urls.pk);
    const pk = await pkResp.arrayBuffer();
    await this.storeProvingKey(circuitId, pk);
  }
}
```

### 2.9 Prover/Verifier Integration

The proof generation is **expensive** (seconds to minutes). The architecture uses a **commit-and-prove** pattern:

```typescript
// src/integration.ts — How it integrates with task execution

/**
 * After executing a work unit, the worker generates a proof.
 *
 * For MVP: We use a simpler approach:
 *   - Input: hash(input)
 *   - Output: hash(output)
 *   - Private: input, output, executionTrace
 *   - Circuit verifies: hash(output) == hash(execute(input))
 *
 * This proves "I ran *some* computation" without revealing data.
 * For stronger guarantees, the circuit must encode the specific program.
 */

export async function proveExecution(
  taskCode: Uint8Array,
  input: unknown,
  output: unknown,
  prover: SnarkProver,
): Promise<{ proof: object; publicSignals: string[] }> {
  // For now: simple arithmetic proof
  // In production: the circuit would verify the specific Wasm execution
  return prover.prove(
    { input: hashInput(input), output: hashOutput(output) },
    { inputRaw: serialize(input), outputRaw: serialize(output) },
  );
}
```

### 2.10 Batch Verification

```typescript
// src/aggregation.ts
/**
 * When many work units from the same task return, we can verify
 * multiple proofs in one batch using recursive Snark verification.
 *
 * For the MVP, verify each proof individually (simpler).
 * For production, aggregate proofs to reduce total verification time.
 *
 * Aggregation approach:
 *   1. Each work unit proof is wrapped in an "aggregation circuit"
 *   2. The aggregation circuit proves: "I verified N sub-proofs"
 *   3. Verifier only checks 1 aggregated proof
 */
export class BatchVerifier {
  private verifier: SnarkVerifier;

  async verifyBatch(proofs: { proof: object; publicSignals: string[] }[]): Promise<boolean> {
    // MVP: verify sequentially
    for (const { proof, publicSignals } of proofs) {
      const valid = await this.verifier.verify(proof, publicSignals);
      if (!valid) return false;
    }
    return true;
  }
}
```

### 2.11 Probabilistic Fallback

Since zk-SNARK proving is computationally expensive, use a **probabilistic fallback**:

```typescript
// src/fallback.ts
/**
 * Strategy: Only require zk-proofs for a random subset of work units.
 * 
 * If a node cheats, it has a 1/N chance of being caught.
 * With N=20, a rational node won't cheat (expected loss > gain).
 */
export class ProbabilisticVerification {
  private verificationProbability: number; // 0.05 = 5% of units get verified

  constructor(probability: number = 0.05) {
    this.verificationProbability = probability;
  }

  shouldVerify(): boolean {
    return Math.random() < this.verificationProbability;
  }

  /**
   * If we suspect cheating, increase verification probability
   * for that node's future units (Bayesian update).
   */
  adjustForSuspicion(nodeId: string, previousViolations: number): void {
    if (previousViolations > 0) {
      this.verificationProbability = Math.min(1, this.verificationProbability * 2);
    }
  }
}
```

### 2.12 Tests

```typescript
describe('SnarkProver', () => {
  it('should generate a valid proof for arithmetic circuit', async () => {
    const prover = new SnarkProver({
      wasmPath: '/circuits/arithmetic.wasm',
      zkeyPath: '/circuits/arithmetic.zkey',
    });
    const { proof, publicSignals } = await prover.prove(
      { op: 0 },
      { a: 5, b: 3 },
    );
    expect(proof).toBeDefined();
    expect(publicSignals).toBeDefined();
  }, 30_000); // may take 10–20s
});

describe('SnarkVerifier', () => {
  it('should verify a valid proof', async () => {
    const vk = await loadVerificationKey('arithmetic');
    const verifier = new SnarkVerifier(vk);
    const { proof, publicSignals } = await generateValidProof();
    const isValid = await verifier.verify(proof, publicSignals);
    expect(isValid).toBe(true);
  });

  it('should reject a tampered proof', async () => {
    const vk = await loadVerificationKey('arithmetic');
    const verifier = new SnarkVerifier(vk);
    const { proof, publicSignals } = await generateTamperedProof();
    const isValid = await verifier.verify(proof, publicSignals);
    expect(isValid).toBe(false);
  });
});

describe('ProbabilisticVerification', () => {
  it('should verify approximately P% of units', () => {
    const pv = new ProbabilisticVerification(0.1);
    const results = Array.from({ length: 10_000 }, () => pv.shouldVerify());
    const verified = results.filter(Boolean).length;
    expect(verified).toBeGreaterThan(800); // 8–12% range
    expect(verified).toBeLessThan(1200);
  });
});
```

---

## 3. Homomorphic Encryption (`packages/homomorphic`)

### 3.1 Purpose
Allow computation on encrypted data without ever decrypting it. The worker node executes on ciphertext and returns an encrypted result that only the requester can decrypt.

### 3.2 File Structure

```
packages/homomorphic/
├── package.json
├── tsconfig.json
├── src/
│   ├── index.ts
│   ├── encryptor.ts         # Encrypt / decrypt operations
│   ├── wasm-executor.ts     # Run Wasm on ciphertexts
│   ├── key-manager.ts       # HE key pair generation & storage
│   ├── types.ts
│   └── config.ts
├── he-wasm/                  # Rust source compiled to Wasm for HE ops
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── bfv.rs            # BFV scheme implementation wrapper
│   │   └── ckks.rs           # CKKS scheme implementation wrapper
│   └── build.sh
├── tests/
│   ├── encryptor.test.ts
│   └── integration.test.ts
```

### 3.3 Technology

For browser-compatible HE, compile **TFHE-rs** or **SEAL** to Wasm:

```bash
# Inside packages/homomorphic/he-wasm/
# Use wasm-pack to compile Rust TFHE implementation to Wasm
cargo new --lib he-wasm
cd he-wasm
cargo add tfhe          # TFHE-rs library
wasm-pack build --target web
```

```rust
// he-wasm/src/lib.rs
use tfhe::prelude::*;
use tfhe::{ConfigBuilder, FheUint8, generate_keys, set_server_key, FheUint32};
use wasm_bindgen::prelude::*;

/// Generate a key pair for homomorphic encryption.
#[wasm_bindgen]
pub fn generate_he_keypair() -> JsValue {
    let config = ConfigBuilder::default().build();
    let (client_key, server_key) = generate_keys(config);
    // Serialize and return as JS object
    serde_wasm_bindgen::to_value(&{
        "clientKey": serialize_key(&client_key),
        "serverKey": serialize_key(&server_key),
    }).unwrap()
}

/// Encrypt a 32-bit integer using the client key.
#[wasm_bindgen]
pub fn encrypt_value(value: u32, client_key_bytes: &[u8]) -> Vec<u8> {
    let client_key: ClientKey = deserialize_key(client_key_bytes);
    let encrypted: FheUint32 = client_key.encrypt(value);
    bincode::serialize(&encrypted).unwrap()
}

/// Add two encrypted values (server-side, no decryption).
#[wasm_bindgen]
pub fn add_encrypted(a_bytes: &[u8], b_bytes: &[u8], server_key_bytes: &[u8]) -> Vec<u8> {
    let server_key: ServerKey = deserialize_key(server_key_bytes);
    set_server_key(server_key);

    let a: FheUint32 = bincode::deserialize(a_bytes).unwrap();
    let b: FheUint32 = bincode::deserialize(b_bytes).unwrap();
    let result = a + b;

    bincode::serialize(&result).unwrap()
}

/// Decrypt a value using the client key.
#[wasm_bindgen]
pub fn decrypt_value(encrypted_bytes: &[u8], client_key_bytes: &[u8]) -> u32 {
    let client_key: ClientKey = deserialize_key(client_key_bytes);
    let encrypted: FheUint32 = bincode::deserialize(encrypted_bytes).unwrap();
    client_key.decrypt(&encrypted)
}
```

### 3.4 TypeScript Wrapper

```typescript
// src/encryptor.ts
import init, { 
  generateHeKeypair, 
  encryptValue, 
  addEncrypted, 
  decryptValue 
} from './he-wasm/pkg/he_wasm.js';

export class HomomorphicEncryptor {
  private initialized = false;

  async init(): Promise<void> {
    if (!this.initialized) {
      await init();
      this.initialized = true;
    }
  }

  async generateKeys(): Promise<{ clientKey: Uint8Array; serverKey: Uint8Array }> {
    await this.init();
    const result = generateHeKeypair() as any;
    return {
      clientKey: new Uint8Array(result.clientKey),
      serverKey: new Uint8Array(result.serverKey),
    };
  }

  async encrypt(value: number, clientKey: Uint8Array): Promise<Uint8Array> {
    await this.init();
    return new Uint8Array(encryptValue(value, clientKey));
  }

  async add(a: Uint8Array, b: Uint8Array, serverKey: Uint8Array): Promise<Uint8Array> {
    await this.init();
    return new Uint8Array(addEncrypted(a, b, serverKey));
  }

  async decrypt(encrypted: Uint8Array, clientKey: Uint8Array): Promise<number> {
    await this.init();
    return decryptValue(encrypted, clientKey);
  }
}
```

### 3.5 Encrypted Execution Flow

```typescript
// src/wasm-executor.ts
/**
 * Execute a Wasm function on encrypted data.
 *
 * Flow:
 * 1. Requester generates HE keypair, sends serverKey + encrypted input to worker
 * 2. Worker loads serverKey, executes computation on ciphertexts
 * 3. Worker returns encrypted result
 * 4. Requester decrypts with clientKey
 *
 * Limitation: HE currently supports only specific operations (add, mul, etc.)
 * General Wasm execution on ciphertext is an open research problem.
 * For MVP: restrict to arithmetic circuits expressible in HE.
 */
export class HomomorphicWasmExecutor {
  private encryptor: HomomorphicEncryptor;

  async executeEncrypted(
    wasmCode: Uint8Array,
    encryptedInputs: Uint8Array[],
    serverKey: Uint8Array,
  ): Promise<Uint8Array> {
    // Load the Wasm module with HE primitives
    const module = await WebAssembly.compile(wasmCode);
    const instance = await WebAssembly.instantiate(module, {
      env: {
        // HE host functions passed to Wasm
        he_add: (a: number, b: number) => addEncrypted(...),
        he_mul: (a: number, b: number) => mulEncrypted(...),
        // ...
      },
    });

    return instance.exports.main(...encryptedInputs);
  }
}
```

### 3.6 Key Distribution Protocol

```typescript
// src/key-manager.ts
export class HeKeyManager {
  private keyStore: Map<string, { clientKey: Uint8Array; serverKey: Uint8Array }> = new Map();

  /**
   * Generate a new key pair for a task.
   * Store clientKey locally, distribute serverKey to workers.
   */
  async generateForTask(taskId: string): Promise<{ serverKey: Uint8Array }> {
    const encryptor = new HomomorphicEncryptor();
    const keys = await encryptor.generateKeys();
    this.keyStore.set(taskId, keys);
    return { serverKey: keys.serverKey };
  }

  /** Requester decrypts result using stored clientKey */
  decryptResult(taskId: string, encrypted: Uint8Array): Promise<number> {
    const keys = this.keyStore.get(taskId);
    if (!keys) throw new Error(`No keys for task ${taskId}`);
    const encryptor = new HomomorphicEncryptor();
    return encryptor.decrypt(encrypted, keys.clientKey);
  }

  /** Clean up keys after task completion */
  destroyKeys(taskId: string): void {
    this.keyStore.delete(taskId);
  }
}
```

### 3.7 Tests

```typescript
describe('HomomorphicEncryptor', () => {
  it('should encrypt and decrypt correctly', async () => {
    const encryptor = new HomomorphicEncryptor();
    const keys = await encryptor.generateKeys();
    const encrypted = await encryptor.encrypt(42, keys.clientKey);
    const decrypted = await encryptor.decrypt(encrypted, keys.clientKey);
    expect(decrypted).toBe(42);
  });

  it('should add encrypted values without decryption', async () => {
    const encryptor = new HomomorphicEncryptor();
    const keys = await encryptor.generateKeys();
    const a = await encryptor.encrypt(10, keys.clientKey);
    const b = await encryptor.encrypt(20, keys.clientKey);
    const encryptedSum = await encryptor.add(a, b, keys.serverKey);
    const decryptedSum = await encryptor.decrypt(encryptedSum, keys.clientKey);
    expect(decryptedSum).toBe(30);
  });

  it('should fail to decrypt with wrong key', async () => {
    const encryptor = new HomomorphicEncryptor();
    const keys1 = await encryptor.generateKeys();
    const keys2 = await encryptor.generateKeys();
    const encrypted = await encryptor.encrypt(42, keys1.clientKey);
    // Decrypting with keys2.clientKey should produce garbage
    const decrypted = await encryptor.decrypt(encrypted, keys2.clientKey);
    expect(decrypted).not.toBe(42);
  });
});
```

---

## 4. Integration: Integrity-Protected Task Lifecycle

```typescript
describe('Full integrity pipeline', () => {
  it('should execute a task, generate proof, verify proof', async () => {
    // Setup mesh + distribution
    const mesh = await createTestMesh(2);
    const distributor = new TaskDistributor(mesh, ...);

    // Task with proof requirement
    const task = createTask({ requireProof: true });

    // Submit → worker executes → generates proof → returns result+proof
    const result = await distributor.submitTask(task);

    // Verify proof
    const verifier = new SnarkVerifier(verificationKey);
    const isValid = await verifier.verify(result.proof, result.publicSignals);
    expect(isValid).toBe(true);
    expect(result.output).toBe(expectedOutput);
  });

  it('should reject tampered results', async () => {
    // Submit task, intercept result, tamper with output
    // Verification should fail
    const result = await submitAndTamper(task);
    const verifier = new SnarkVerifier(verificationKey);
    const isValid = await verifier.verify(result.proof, result.publicSignals);
    expect(isValid).toBe(false);
  });
});
```

---

## 5. Development Workflow for Phase 3

### Step-by-step build order

| Week | Focus | Components |
|------|-------|-----------|
| **10** | zk-SNARK Foundation | Circom setup, arithmetic circuit, proving key generation, snarkjs integration |
| **11** | Prover & Verifier | Prover (worker), verifier (requester), key management, batch verification |
| **12** | Homomorphic Encryption | Rust TFHE-wasm compilation, encryptor/decryptor, encrypted execution |
| **13** | Integration + Fallback | Probabilistic verification strategy, full integrity pipeline e2e, benchmarks |

### Circuit Setup (Week 10, Day 1)

```bash
# Download Powers of Tau ceremony file (phase 1)
wget https://hermez.s3.amazonaws.com/powersOfTau28_hez_final_12.ptau

# Compile circuit
circom circuits/arithmetic.circom --r1cs --wasm --sym

# Generate proving key (phase 2)
snarkjs groth16 setup arithmetic.r1cs powersOfTau28_hez_final_12.ptau arithmetic.zkey

# Export verification key
snarkjs zkey export verificationkey arithmetic.zkey verification_key.json

# Export Solidity verifier (for on-chain verification)
snarkjs zkey export solidityverifier arithmetic.zkey verifier.sol
```

### Performance benchmarks to run

| Operation | Target | Measurement |
|-----------|--------|-------------|
| Proving time (arithmetic) | < 5s | snarkjs timing |
| Verification time | < 500ms | snarkjs timing |
| Proof size | < 1KB | bytes |
| HE encryption | < 10ms per value | performance.now() |
| HE addition on ciphertext | < 50ms | performance.now() |
| HE key generation | < 2s | performance.now() |

---

## 6. Deliverables Checklist (End of Phase 3)

- [ ] Circom arithmetic circuit compiled and tested
- [ ] Powers of Tau ceremony file downloaded
- [ ] Proving key and verification key generated
- [ ] `SnarkProver` generates valid proofs from worker nodes
- [ ] `SnarkVerifier` verifies proofs on requester side (< 500ms)
- [ ] `KeyManager` stores/loads keys via IndexedDB
- [ ] `BatchVerifier` verifies multiple proofs
- [ ] `ProbabilisticVerification` fallback (5% random check)
- [ ] Rust HE library compiled to Wasm via wasm-pack
- [ ] `HomomorphicEncryptor` encrypts/decrypts integers
- [ ] Homomorphic addition on ciphertexts works
- [ ] `HeKeyManager` for key generation per task
- [ ] Integration test: task with zk-proof works end-to-end
- [ ] Integration test: tampered proof is rejected
- [ ] Integration test: encrypted computation works
- [ ] Performance benchmarks documented
- [ ] All packages pass `pnpm build` and `pnpm test`
