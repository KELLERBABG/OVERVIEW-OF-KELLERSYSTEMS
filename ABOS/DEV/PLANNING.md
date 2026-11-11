# Atmospheric Broadcast OS (ABOS) — Comprehensive Architecture & Planning Documentation

> **Project Directory:** `ABOS/`  
> **Primary Language / Toolchain:** `Rust`  
> **Verification Command:** `cargo test --workspace --quiet`  
> **Reference Sources:** `KELLERSYSTEMS Spine.md` · `PaySlip.canvas` · `Life Work.canvas`

---

## 1. Executive Spine & System Overview

**Atmospheric Broadcast OS (ABOS)** — A software-defined radio (SDR) operating system written in Rust that enables covert, resilient, long-range communication without routing tables by exploiting ionospheric Near-Vertical Incidence Skywave (NVIS) propagation.

### Current Engineering Baseline
A 13-crate Rust SDR workspace implementing OFDM/DSSS/FHSS physical layers, LDPC forward error correction, DTN bundle store-and-forward, and simulated NVIS skywave channels. While DSP, FEC, and protocol crates pass tests, the ionospheric sounding modules (`fo_f2.rs`, `meteor.rs`, `nvis.rs`, `propagation.rs`) and cognitive jammer detection (`jammer_detect.rs`) are thin 3–9 line stubs, and hardware SDR surfaces remain mock-only.

---

## 2. Canonical Specifications & Mathematical Invariants (`PaySlip.canvas` / `Spine`)

The following 12 architectural pillars and formal invariants define the target engineering state of the system:

- **Skywave Propagation (Ionosphärischer Funk):** Die Mathematik der Reflexion von HF-Signalen an der Ionosphäre, um die Erdkrümmung zu überwinden (NVIS-Technik).
    
- **Software Defined Radio (SDR):** Programmierung von Funkprotokollen rein in Software (Rust), um Hardware-Beschränkungen zu umgehen.
    
- **OFDM (Orthogonal Frequency-Division Multiplexing):** Effiziente Aufteilung digitaler Daten auf tausende eng beieinander liegende Funkfrequenzen.
    
- **Cognitive Radio:** KI-gesteuerte Frequenzwahl, die automatisch Lücken im Funkspektrum findet und staatliche Störsender (Jamming) umgeht.
    
- **Burst-Transmission:** Senden von hochkomprimierten Datenpaketen in Millisekunden-Bruchteilen, um Peilung und Lokalisierung zu erschweren.
    
- **Bit-Interleaved Coded Modulation (BICM):** Robuste Fehlerkorrektur für instabile Funkverbindungen durch atmosphärische Störungen.

---

## 3. Theoretical Foundations & Protocol Mechanics (`Life Work.canvas`)

Extracted architectural derivations, mathematical formulations, and state-machine specifications from the master canvas:

### [Atmospheric Broadcast OS] Specification Node
> Maxwellsche Gleichungen & Elektrodynamik
> 
> Die Gleichungen beschreiben, wie sich elektromagnetische Wellen im Raum ausbreiten. Ich muss insbesondere die Wellengleichung anschauen

### [Atmospheric Broadcast OS] Specification Node
> Ionosphärenphysik, Plasmadichte & NVIS
> 
> Die Ionosphäre besteht aus Gasen die durch UV-Strahlung ionisiert wurden. Die verhält sich für Radiowellen wie ein Plasma. Ob eine Welle reflektiert oder ins All entweicht, hängt von der Plasmafrequenz ab

### [Atmospheric Broadcast OS] Specification Node
> Ionospheric Sounding Integration
> 
> Die Elektronendichte ändert sich je nach Wetter, deswegen muss ABOS dynamisch reagieren

### [Atmospheric Broadcast OS] Specification Node
> Meteor Scatter Sync & Phoenix-Fenster
> 
> Meteoren hinterlassen eine hochkonzentrierte, kurzlebige Ionisationsspur aus freien Elektronen

### [Atmospheric Broadcast OS] Specification Node
> Abtasttheorem, Fouriertransformation (FFT/IFFT) & OFDM

### [Atmospheric Broadcast OS] Specification Node
> Digital Down Conversion & Decimation
> 
> Die ADC-Chips des SDRs tasten mit hunderten Megahertz ab. Das erzeugt massive Datenströme, die die CPU überlasten würde

### [Atmospheric Broadcast OS] Specification Node
> Phase-Locked Loops & Costas Loops
> 
> Da Quarzoszillatoren von Sendern und Empfängern ungenau sind, laufen die Frequenz und Phase immer ein wenig auseinander. Eine Costas Loop ist eine spezialisierte PLL, die im Empfänger die Phasenverschiebung des I/Q-Signals misst und den lokale Oszillator durchgehend nachregelt. Ohne Costas Loop rotiert die Konstellation im Empfänger, und die Bits können nicht demaskiert werden

### [Atmospheric Broadcast OS] Specification Node
> Pulse Shaping
> 
> Ein abrupter Wechsel zwischen Signalzuständen erzeugt theoretisch unendliche breite Frequenzen im Spektrum. Um die Bandbreite zu begrenzen und ISI zu verhindern, gehen die Symbole durch ein RRC-Filter (Root-Raised Cosine). Es glättet die Übergänge im Zeitbereich

### [Atmospheric Broadcast OS] Specification Node
> Ein SDR verarbeitet keine Spannungskurven sondern In-Phase (I) und Quadrature (Q) Komponenten. Ein Signal wird dargestellt wie folgt:
> $$s(t) = I(t) + j \cdot Q(t)$$
> Dadurch kann ich Amplitude und Phase eines Signals gleichzeitig manipulieren.

### [Atmospheric Broadcast OS] Specification Node
> Nyquist-Shannon:
> - Um ein Signal mit der Bandbreite $B$ verlustfrei zu digitalisieren muss ich mindestens $f_s > 2B$ abtasten

### [Atmospheric Broadcast OS] Specification Node
> Umsetzung:
> - Ich integriere einen Empfänger, eine Ionosonde oder lese Echtzeit-Datenströme aus um die kritische Frequenz der F2-Schicht zu berechnen

### [Atmospheric Broadcast OS] Specification Node
> Physik:
> - Diese Spuren reflektieren Frequenzen von 30-100 Mhz, die die Ionosphäre durchschlagen würden. Diese verbrennen in einigen millisekunden bis wenige Sekunden, erreichen aber Reichweiten von bis zu 2000km

### [Atmospheric Broadcast OS] Specification Node
> DDC: 
> - Das Signal wird mathematisch mit einem Oszillator gemischt um es ins Basisband zu verschieben

### [Atmospheric Broadcast OS] Specification Node
> $$\nabla^2 \mathbf{E} - \mu \epsilon \frac{\partial^2 \mathbf{E}}{\partial t^2} = 0$$
> 
> Wenn ich Signale über SDR emittiere, steuer ich die Stromdichte $\mathbf{J}$ in der Antenne um ein elektromagnetisches Feld zu erzeugen.

### [Atmospheric Broadcast OS] Specification Node
> OFDM:
> - Statt Daten auf einer Frequenz zu senden, teile ich die Bandbreite in tausende Subcarrier auf. da nutze ich die IFFT beim Senden, und FFT fürs empfangen. OFDM ist sehr robust gegen frequenzselektives Fading

### [Atmospheric Broadcast OS] Specification Node
> $$f_p \approx 9 \sqrt{N_e}$$
> Wobei $N_e$ die Elektronendichte pro Kubikmeter ist.
> - Skywave Propagation: Wenn meine Sendefrequenz $f$< kritische Frequenz $f_c$ der Ionosphäre wird die Welle reflektiert
> - NIVS: Ich Strahle das Signal mit 80-90 Grad nach oben, es reflektiert in der F-Schicht der Ionosphäre und regnet kreisförmig herunter.

### [Atmospheric Broadcast OS] Specification Node
> Decimation:
> - Ein Tiefpassfilter schneidet unerwünschte Frequenzen ab, und danach jeder $n-$te Abtastwert behalten. Da muss ich eine SIMD-optimierte FIR-Filter bauen um den Datenstrom auf eine handhabbare Samplerate zu reduzieren

### [Atmospheric Broadcast OS] Specification Node
> Logik:
> - liegt $f_0F2$ bei 7Mhz, darf meine Übertragungsfrequenz für NVIS nicht darüber sein. Es muss die Trägerfrequenz vom SDR zur Laufzeit anpassen

### [Atmospheric Broadcast OS] Specification Node
> Opportunistic Scheduling:
> - Mein OS darf nicht immer senden. Es lauscht auf ein Baken-Signal. Sobald ein Peak im SNR erkannt wird, schaltet der Scheduler in den Übertragungsmodus und jagt die Handshake-Shards mit maximaler Symbolrate durch bevor die Spur kollabiert

### [Atmospheric Broadcast OS] Specification Node
> Asynchronous Buffer-Bounce
> 
> Dieses Protokoll bricht den Zwang, dedizierte Empfängeradressen zu kennen. Die Atmosphäre selbst wird zum RAM-Speicher

### [Atmospheric Broadcast OS] Specification Node
> Cyclostationary Feature Detection & Signal-Tarnung
> 
> Militärische Überwachungssysteme nutzen nicht nur Energie-Detektoren, aber analysieren auch statistische Muster, um Künstichle Signale im Rauschen zu finden.

### [Atmospheric Broadcast OS] Specification Node
> Cognitive Radio
> 
> Das System scannt über ein FFT-Fesnter permanent das Funkspektrum. Erkennt die KI/Heuristik einen staatlichen Jammer oder ein primäres Funksignal, weicht das OS auf freie Spektrumslücken aus (White Space)

### [Atmospheric Broadcast OS] Specification Node
> Antenna Array Beamforming
> 
> Wenn ich mehrere Antennen nebeneinander aufstelle, kann ich die Phasenlage des Signals an jeder Antenne verschieben. Die Wellenfronten überlagern sich konstruktiv in eine Richtung und destruktiv in alle anderen Richtungen. Dadurch ensteht ein schmahler Richtstrahl. Ein Angreifer der wenige Grad abseits dieser Linie ist, bekommt nichts

### [Atmospheric Broadcast OS] Specification Node
> Doppler-Shift:
> 
> Bei Meteor Scatter oder sich bewegenden Ionosphärenschichten verschiebt sich die Frequenzt. Das OS muss die Trägerfrequenz kontinuierlich Nachziehen über einen Frequenz-Diskrimentator im Empfänger

### [Atmospheric Broadcast OS] Specification Node
> Delay Tolerant Networking:
> 
> Das OS nutz Store-and-Forward. Daten werden in Bundels gekapselt, lokal im persistent Speicher abgelegt und erst dann emittiert, wenn ein Übertragungsfenster offen ist

---

## 4. Codebase Architecture & Module Inventory

Total tracked source and configuration files in `ABOS/`: **87**

| File Path | Lines | Architectural Role |
| :--- | :---: | :--- |
| `Cargo.toml` | 37 | Build / Package Manifest |
| `abos-cli/Cargo.toml` | 25 | Build / Package Manifest |
| `abos-cli/src/lib.rs` | 488 | Core Implementation Module |
| `abos-cli/src/main.rs` | 5 | Core Implementation Module |
| `abos-cognitive/Cargo.toml` | 12 | Build / Package Manifest |
| `abos-cognitive/src/adapt.rs` | 32 | Core Implementation Module |
| `abos-cognitive/src/jammer_detect.rs` | 7 | Core Implementation Module |
| `abos-cognitive/src/lib.rs` | 4 | Core Implementation Module |
| `abos-cognitive/src/scanner.rs` | 26 | Core Implementation Module |
| `abos-cognitive/src/whitespace.rs` | 21 | Core Implementation Module |
| `abos-common/Cargo.toml` | 21 | Build / Package Manifest |
| `abos-common/src/complex.rs` | 44 | Core Implementation Module |
| `abos-common/src/crypto.rs` | 125 | Core Implementation Module |
| `abos-common/src/error.rs` | 43 | Core Implementation Module |
| `abos-common/src/lib.rs` | 5 | Core Implementation Module |
| `abos-common/src/pn_gen.rs` | 82 | Core Implementation Module |
| `abos-common/src/types.rs` | 90 | Core Implementation Module |
| `abos-common/tests/integration_test.rs` | 155 | Verification & Test Suite |
| `abos-dsp/Cargo.toml` | 11 | Build / Package Manifest |
| `abos-dsp/src/agc.rs` | 42 | Core Implementation Module |
| `abos-dsp/src/costas.rs` | 43 | Core Implementation Module |
| `abos-dsp/src/ddc.rs` | 29 | Core Implementation Module |
| `abos-dsp/src/decimation.rs` | 61 | Core Implementation Module |
| `abos-dsp/src/fft.rs` | 24 | Core Implementation Module |
| `abos-dsp/src/iq_correct.rs` | 33 | Core Implementation Module |
| `abos-dsp/src/lib.rs` | 9 | Core Implementation Module |
| `abos-dsp/src/ofdm.rs` | 78 | Core Implementation Module |
| `abos-dsp/src/pulse_shape.rs` | 64 | Core Implementation Module |
| `abos-dsp/src/timing.rs` | 54 | Core Implementation Module |
| `abos-dsp/tests/integration_test.rs` | 153 | Verification & Test Suite |
| `abos-fec/Cargo.toml` | 15 | Build / Package Manifest |
| `abos-fec/src/crc.rs` | 35 | Core Implementation Module |
| `abos-fec/src/interleaver.rs` | 58 | Core Implementation Module |
| `abos-fec/src/ldpc.rs` | 327 | Core Implementation Module |
| `abos-fec/src/lib.rs` | 4 | Core Implementation Module |
| `abos-fec/src/soft_decision.rs` | 58 | Core Implementation Module |
| `abos-fec/tests/integration_test.rs` | 119 | Verification & Test Suite |
| `abos-gui/Cargo.toml` | 19 | Build / Package Manifest |
| `abos-gui/src/lib.rs` | 252 | Core Implementation Module |
| `abos-gui/src/view.rs` | 198 | Core Implementation Module |
| `abos-hal/Cargo.toml` | 11 | Build / Package Manifest |
| `abos-hal/src/dma.rs` | 113 | Core Implementation Module |
| `abos-hal/src/gpio.rs` | 26 | Core Implementation Module |
| `abos-hal/src/lib.rs` | 4 | Core Implementation Module |
| `abos-hal/src/sdr.rs` | 445 | Core Implementation Module |
| `abos-hal/src/time.rs` | 38 | Core Implementation Module |
| `abos-iono/Cargo.toml` | 11 | Build / Package Manifest |
| `abos-iono/src/fo_f2.rs` | 3 | Core Implementation Module |
| `abos-iono/src/lib.rs` | 5 | Core Implementation Module |
| `abos-iono/src/meteor.rs` | 9 | Core Implementation Module |
| `abos-iono/src/nvis.rs` | 8 | Core Implementation Module |
| `abos-iono/src/propagation.rs` | 6 | Core Implementation Module |
| `abos-iono/src/sounder.rs` | 31 | Core Implementation Module |
| `abos-phy/Cargo.toml` | 12 | Build / Package Manifest |
| `abos-phy/src/beamforming.rs` | 88 | Core Implementation Module |
| `abos-phy/src/burst.rs` | 164 | Core Implementation Module |
| `abos-phy/src/dsss.rs` | 169 | Core Implementation Module |
| `abos-phy/src/fhss.rs` | 77 | Core Implementation Module |
| `abos-phy/src/lib.rs` | 6 | Core Implementation Module |
| `abos-phy/src/modem.rs` | 235 | Core Implementation Module |
| *...and 27 additional files* | — | — |

---

## 5. Gap Analysis & Production-Readiness Audit

- **Ionospheric Physics & Sounding (`abos-iono`):** `fo_f2.rs` (3 lines), `meteor.rs` (9 lines), `nvis.rs` (8 lines), and `propagation.rs` (6 lines) are placeholder functions that do not yet compute Appleton-Hartree ordinary/extraordinary wave refraction, critical frequency $f_oF2$ secant law MUF ($f_{\text{MUF}} = f_oF2 \cdot \sec\theta_i$), or meteor-scatter burst windows.
- **Cognitive Anti-Jamming (`abos-cognitive`):** `jammer_detect.rs` (7 lines) and `whitespace.rs` (21 lines) lack spectral kurtosis / cyclostationary jammer classification and dynamic notch/frequency-hopping evasion schedules.
- **Stealth Cyclostationary Masking (`abos-stealth`):** `mask_cyclo.rs` (15 lines) and `amp_dither.rs` (26 lines) need full sub-noise-floor power spectral density (PSD) bounding below $-15\text{ dB}$ SNR so transmissions remain statistically indistinguishable from thermal noise.
- **Antenna Array Beamforming (`abos-phy`):** `beamforming.rs` (88 lines) needs elevation-steered NVIS array factor synthesis ($\theta \in [70^\circ, 90^\circ]$) coupled to real-time ionospheric height estimates.

---

## 7. Actionable Development & Verification Queue

Each task below is synchronized with `DEV/ROADMAP.md` and `DEV/TASKS.json` for automated or manual execution:

| Task ID | Title | Target Files | Verification Command |
| :---: | :--- | :--- | :--- |
| **TASK-01** | Implement Appleton-Hartree NVIS propagation & critical frequency foF2 solver | `abos-iono/src/fo_f2.rs`, `abos-iono/src/nvis.rs`, `abos-iono/src/propagation.rs` | `cargo test -p abos-iono --quiet` |
| **TASK-02** | Implement meteor-scatter ionization trail burst window estimator | `abos-iono/src/meteor.rs` | `cargo test -p abos-iono --quiet` |
| **TASK-03** | Implement cognitive radio jammer classification and whitespace hopping | `abos-cognitive/src/jammer_detect.rs`, `abos-cognitive/src/whitespace.rs`, `abos-cognitive/src/adapt.rs` | `cargo test -p abos-cognitive --quiet` |
| **TASK-04** | Harden cyclostationary feature masking and sub-noise-floor stealth invariants | `abos-stealth/src/mask_cyclo.rs`, `abos-stealth/src/amp_dither.rs`, `abos-stealth/src/phase_noise.rs` | `cargo test -p abos-stealth --quiet` |
| **TASK-05** | Couple NVIS elevation beamforming to ionospheric sounder feedback | `abos-phy/src/beamforming.rs` | `cargo test -p abos-phy --quiet` |
| **TASK-06** | End-to-end asynchronous buffer-bounce DTN mesh verification | `abos-protocol/src/bounce.rs`, `abos-tests/tests/e2e_roundtrip.rs` | `cargo test -p abos-tests --quiet` |
