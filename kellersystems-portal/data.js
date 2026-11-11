const REPOSITORIES = [
    {
        "name": "ABOS",
        "raw_name": "ABOS",
        "description": "Atmospheric Broadcast OS: an SDR and radio-level fallback for internet access, carried by Vantablack as an opt-in submodule",
        "url": "https://abos.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-09-26T19:28:48Z"
    },
    {
        "name": "AeroResonance",
        "raw_name": "AeroResonance",
        "description": "Atmospheric electromagnetic and acoustic resonance energy harvester",
        "url": "https://resonance.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-09-06T00:00:00Z"
    },
    {
        "name": "Vantablack",
        "raw_name": "Vantablack",
        "description": "Post-quantum decentralized WAN mesh network with erasure-sharded onion routing",
        "url": "https://vantablack.kellersystems.dev",
        "github_url": "https://github.com/KELLERBABG/Vantablack",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-10-05T00:00:00Z"
    },
    {
        "name": "Quantum Entanglement Link",
        "raw_name": "QEL",
        "description": "Quantum communication network simulation stack: density-matrix states and noise, QKD and error-correcting codes, and fidelity-constrained repeater routing over a mesh topology",
        "url": "https://github.com/KELLERBABG/QEL",
        "github_url": "https://github.com/KELLERBABG/QEL",
        "language": "Python",
        "category": "Crypto",
        "updated": "2026-10-05T00:00:00Z"
    },
    {
        "name": "AeroDose",
        "raw_name": "AeroDose",
        "description": "Occupational wildfire PM2.5 shift dose and compliance ledger",
        "url": "https://aero.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-09-06T17:23:52Z"
    },
    {
        "name": "Algorithmic Supply Chain Arbitrage",
        "raw_name": "Algorithmic-Supply-Chain-Arbitrage",
        "description": "Algorithmic Supply Chain Arbitrage and multi-modal freight routing engine",
        "url": "https://supply.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-09-06T17:58:58Z"
    },
    {
        "name": "Ambient Thermal Arbitrage",
        "raw_name": "Ambient-Thermal-Arbitrage",
        "description": "Ambient Thermal Arbitrage and virtual battery energy management system",
        "url": "https://ambient.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-09-06T17:58:59Z"
    },
    {
        "name": "AmtFlow",
        "raw_name": "AmtFlow",
        "description": "Kostenloser, lokaler Assistent fuer deutsche Behoerdenbriefe mit OCR-Texterkennung, Fristberechnung, 20 Sprachen und DIN 5008 Antwortschreiben.",
        "url": "https://amt.kellersystems.dev",
        "language": "JavaScript",
        "category": "General",
        "updated": "2026-09-06T17:23:49Z"
    },
    {
        "name": "AquaMOF",
        "raw_name": "AquaMOF",
        "description": "Passive atmospheric water harvester using MOF-303 isotherm thermodynamics",
        "url": "https://aquamof.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-09-06T17:23:48Z"
    },
    {
        "name": "Aureal Watermark",
        "raw_name": "Aureal-Watermark",
        "description": "Audio watermarking for anti-theft and AI detection",
        "url": "https://aureal.kellersystems.dev",
        "language": "HTML",
        "category": "General",
        "updated": "2026-09-06T08:27:40Z"
    },
    {
        "name": "AutoPoD",
        "raw_name": "AutoPoD",
        "description": "Autonomous Proof-of-Damage layer with cryptographic incident audit",
        "url": "https://pod.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-08-25T08:13:41Z"
    },
    {
        "name": "BauLens",
        "raw_name": "BauLens",
        "description": "Deterministic German and EU building code compliance validation engine",
        "url": "https://lens.kellersystems.dev",
        "language": "JavaScript",
        "category": "General",
        "updated": "2026-09-06T17:23:33Z"
    },
    {
        "name": "CityOS",
        "raw_name": "CityOS",
        "description": "Decentralized civic infrastructure and urban resource routing engine",
        "url": "https://city.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-09-06T17:23:43Z"
    },
    {
        "name": "Decentralized Sovereign Credit Mesh",
        "raw_name": "Decentralized-Sovereign-Credit-Mesh",
        "description": "Decentralized Sovereign Credit Mesh using zero-knowledge proofs and web-of-trust vouching",
        "url": "https://credit.kellersystems.dev",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-09-06T17:59:02Z"
    },
    {
        "name": "Differential Privacy Synthesizer",
        "raw_name": "Differential-Privacy-Synthesizer",
        "description": "Differential Privacy Synthesizer with bounded epsilon privacy leak budgets",
        "url": "https://dps.kellersystems.dev",
        "language": "Python",
        "category": "Crypto",
        "updated": "2026-09-06T17:59:01Z"
    },
    {
        "name": "FluidIP",
        "raw_name": "FluidIP",
        "description": "Decoupled Fluid IP routing protocol and packet mesh",
        "url": "https://patent.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-09-06T17:23:34Z"
    },
    {
        "name": "Formal Verification",
        "raw_name": "Formal-Verification",
        "description": "High-assurance engineering codebase and research implementation.",
        "url": "https://veritas.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-08-25T08:17:23Z"
    },
    {
        "name": "HoloForge",
        "raw_name": "HoloForge",
        "description": "Holographic tomographic volumetric 3D printing suite (Tomo-H-VAM)",
        "url": "https://holoforge.kellersystems.dev",
        "language": "Python",
        "category": "General",
        "updated": "2026-09-06T17:23:36Z"
    },
    {
        "name": "Homomorphic Database",
        "raw_name": "Homomorphic-Database",
        "description": "Homomorphic Encrypted Database (FHE-DB): Relational queries on ciphertext",
        "url": "https://fhe.kellersystems.dev",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-09-06T17:59:04Z"
    },
    {
        "name": "KELLER-OS",
        "raw_name": "KELLER-OS",
        "description": "High-assurance engineering codebase and research implementation.",
        "url": "https://kernel.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-09-06T17:23:39Z"
    },
    {
        "name": "ClauseLens",
        "raw_name": "ClauseLens",
        "description": "Automated contract clause contradiction and risk scoring engine",
        "url": "https://clause.kellersystems.dev",
        "language": "JavaScript",
        "category": "General",
        "updated": "2026-09-06T17:23:42Z"
    },
    {
        "name": "MuonScope Tomography",
        "raw_name": "MuonScope-Tomography",
        "description": "MuonScope\u2122: Cosmic-Ray Muon Scattering Tomography Digital Twin, 3D Inversion Engine & Silicon FPGA Architecture",
        "url": "https://muon.kellersystems.dev",
        "language": "Python",
        "category": "General",
        "updated": "2026-09-06T17:23:32Z"
    },
    {
        "name": "MycoSwarm",
        "raw_name": "MycoSwarm",
        "description": "Mycelium biological network computation and bio-routing simulator",
        "url": "https://bio.kellersystems.dev",
        "language": "TypeScript",
        "category": "OS",
        "updated": "2026-09-06T17:23:51Z"
    },
    {
        "name": "OP-Roboter",
        "raw_name": "OP-Roboter",
        "description": "Robotic surgical teleoperation and real-time kinematic safety kernel",
        "url": "https://kinematics.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-09-06T17:23:37Z"
    },
    {
        "name": "PacePulse",
        "raw_name": "PacePulse",
        "description": "Zero-latency medical telemetry and cardiac rhythm monitoring platform",
        "url": "https://pulse.kellersystems.dev",
        "language": "Python",
        "category": "General",
        "updated": "2026-08-25T08:14:23Z"
    },
    {
        "name": "PlaneOS",
        "raw_name": "PlaneOS",
        "description": "Avionics real-time operating system and flight envelope safety supervisor",
        "url": "https://plane.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-08-25T08:14:46Z"
    },
    {
        "name": "Planetary Entropy Mining",
        "raw_name": "Planetary-Entropy-Mining",
        "description": "Certificates grounded in a measured window of physical noise, with a zero-knowledge proof of honest capture that hides the node's location",
        "url": "https://pem.kellersystems.dev",
        "language": "Python",
        "category": "Crypto",
        "updated": "2026-09-06T17:59:03Z"
    },
    {
        "name": "Smart Matter",
        "raw_name": "Smart-Matter",
        "description": "High-assurance engineering codebase and research implementation.",
        "url": "https://smart-matter.kellersystems.dev",
        "language": "Rust",
        "category": "Rust",
        "updated": "2026-08-16T12:17:17Z"
    },
    {
        "name": "SMPC Dark Pool",
        "raw_name": "SMPC-Dark-Pool",
        "description": "Secure Multi-Party Computation Dark Pool with differential privacy guarantees",
        "url": "https://enclave.kellersystems.dev",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-09-06T17:59:03Z"
    },
    {
        "name": "SybilGuard",
        "raw_name": "SybilGuard",
        "description": "Zero-knowledge Sybil-resistant Proof-of-Personhood biometric protocol",
        "url": "https://citadel.kellersystems.dev",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-09-06T17:23:50Z"
    },
    {
        "name": "SymEx EVM",
        "raw_name": "SymEx-EVM",
        "description": "Symbolic execution engine for EVM bytecode vulnerability detection and SMT verification",
        "url": "https://axiom.kellersystems.dev",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-09-06T17:23:49Z"
    },
    {
        "name": "TerraFlux",
        "raw_name": "TerraFlux",
        "description": "Precision autonomous soil moisture micro-irrigation controller",
        "url": "https://terra.kellersystems.dev",
        "language": "Python",
        "category": "General",
        "updated": "2026-08-25T08:12:54Z"
    },
    {
        "name": "TradeShield",
        "raw_name": "TradeShield",
        "description": "Zero-fraud DEX wash-trading detector and volume integrity ledger",
        "url": "https://trade.kellersystems.dev",
        "language": "TypeScript",
        "category": "Crypto",
        "updated": "2026-08-25T08:13:17Z"
    },
    {
        "name": "TransitOS",
        "raw_name": "TransitOS",
        "description": "Autonomous public transit fleet scheduling and demand-responsive routing kernel",
        "url": "https://transit.kellersystems.dev",
        "language": "Rust",
        "category": "OS",
        "updated": "2026-08-25T08:32:26Z"
    },
    {
        "name": "Zero Knowledge Genome",
        "raw_name": "Zero-Knowledge-Genome",
        "description": "Zero-Knowledge Genome: Private biological sequence matching and genomic proofs",
        "url": "https://zkg.kellersystems.dev",
        "language": "Rust",
        "category": "Crypto",
        "updated": "2026-09-06T17:59:03Z"
    }
];
