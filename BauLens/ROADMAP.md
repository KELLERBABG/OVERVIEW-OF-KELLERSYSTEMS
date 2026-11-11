# BauCheck / BauLens — Commercialization & Product Roadmap (Target 9/10 & 10/10)

**Project:** BauLens / BauCheck — Regulatory German Building Code & CAD/BIM Dossier Engine  
**Current Version:** `0.10.0` (Desktop Licensing Unreleased)  
**Current Readiness Score:** **7.5/10 (Advanced Desktop Product)**  
**Target Milestone:** **9/10 (Commercially Shippable / Zero-Friction Product)**  
**Long-Term Milestone:** **10/10 (Field-Tested Production with Architecture Firm Maintenance)**

---

## 1. Executive Summary & Philosophy

BauLens provides precision German building code compliance checks (LBO § 6 Abstandsflächen, B-Plan parsing, XPlanung, and DIN A4 dossier PDF generation). The software exists as both a tested CLI engine and a standalone Tauri v2 desktop application.

To reach **9/10**, the project must transition from local builds to an automated, code-signed release pipeline, activate live Polar checkout links, and deliver prebuilt installers directly from the public download hub.

---

## 2. Current Verified Baseline (v0.10.0)

- **Regulatory Breadth:** Full coverage of standard German state building codes (MBO, 16 Bundesländer, Austrian OIB, Swiss IVHB standards).
- **DIN A4 Dossier Engine:** Native, dependency-free vector PDF writer (`src/dossierpdf.js`, `src/pdf.js`) with Adobe AFM typography metrics and deterministic SHA-256 seal generation.
- **Cadastral Footprints:** Geometry computed from actual plan parameters (`GRZ_ist × Grundstücksfläche`).
- **Desktop Architecture:** Tauri v2 shell with dedicated modular inject scripts (`boot.js`, `licensing-ui.js`, `shell.js`, `sources.mjs`).
- **Trial & Entitlements:** 14-day desktop trial state management (`src/licensing.js`), Polar API validation with 30-day offline grace, and architectural UI/command lockout (`assertUnlocked()`).

---

## 3. Milestone 1: Automated Release Pipeline & Code Signing (Target 9/10)

```
[git tag v0.10.1] ───> GitHub Actions Runner
                         ├──> Windows: tauri build ──> signed BauLens-Setup.exe / .msi
                         ├──> macOS:   tauri build ──> notarized BauLens.dmg
                         └──> Release Assets Published ──> downloads/index.html updated
```

- [ ] **Commit & Merge Licensing Branch:**
  - Verify and merge the unreleased licensing inject architecture and test suites into `main`.
- [ ] **GitHub Actions Tauri v2 Matrix:**
  - Establish `.github/workflows/desktop-release.yml` triggered on version tags (`v*`).
  - Automated Windows build emitting `.msi` and `.exe` with embedded application icon.
  - Automated macOS build emitting `.dmg` for both Apple Silicon (`aarch64`) and Intel (`x86_64`).
- [ ] **Windows Code Signing:**
  - Configure Authenticode certificate signing in CI to prevent Windows SmartScreen warnings upon user download.

---

## 4. Milestone 2: Live Payment & Storefront Activation (Target 9/10)

- [ ] **Production Polar Checkout:**
  - Replace sandbox/portal stubs with live Polar products in `site.config.json`:
    - Commercial Solo: €49/mo or €490/yr.
    - Architecture Studio: €199/mo or €1.990/yr.
  - Ensure the in-app "Lizenz erwerben" button opens the customer portal with pre-filled metadata.
- [ ] **Downloads Hub Integration:**
  - Wire `downloads/index.html` to query GitHub API for the latest release assets, rendering direct 1-click download buttons with file size and SHA-256 checksum badges.

---

## 5. Milestone 3: Engine Resilience & Format Hardening (Target 9/10)

- [ ] **DWG to DXF Ingestion Fallback:**
  - While DXF parsing is native and robust, many German architecture offices default to proprietary AutoCAD `.dwg`. Provide a documented, zero-telemetry conversion helper or native header extractor.
- [ ] **XPlanung 6.0 Full Compatibility:**
  - Update XPlanung / GeoJSON parser to support recent municipal extensions across Munich, Berlin, Hamburg, and Frankfurt.
- [ ] **Dossier Batch Print Automation:**
  - Expose CLI batch mode for planning offices: `baucheck --batch ./projekte/ --export-all-pdf ./befunde/`.

---

## 6. Milestone 4: Customer Documentation & Onboarding (Target 9/10)

- [ ] **Live Tutorial & Video Walkthrough:**
  - Embed a 2-minute video walkthrough on `baulens.kellersystems.dev` demonstrating:
    1. Drag-and-drop DXF plan loading.
    2. Real-time 3D setback envelope inspection.
    3. Generating an official DIN A4 dossier PDF.
- [ ] **Regulatory Assurance Paper:**
  - Publish `docs/LEGAL_VALIDITY.md` outlining how BauLens seals fulfill evidentiary requirements for preliminary municipal building inquiries (*Bauvoranfrage*).

---

## 7. Milestone 5: Path to 10/10 (Field-Tested Architecture Practice)

10/10 requires real-world deployment across professional German architectural offices:

1. **Pilot Office Deployment:**
   - Deploy BauLens in 10 architecture firms across Germany, Austria, and Switzerland.
2. **Official Bauamt Inquiries:**
   - Validate 50+ real planning submissions where municipal building authorities (*Bauämter*) accepted BauLens dossier PDFs as supplementary compliance evidence.
3. **Continuous Regulatory Tracking:**
   - Establish a 48-hour response protocol for updates to state building laws (*LBO Novellierungen*).
