# Production Readiness Report

**Subject:** Aureal Watermark, package version 0.2.7
**Commit reviewed:** `30c311f` (`main`, 2026-10-21). The `v0.2.7` tag is 14 commits behind this commit (`git describe`: `v0.2.7-14-g30c311f`).
**Report date:** 2026-11-02
**Reviewer:** local verification pass in this checkout. Windows 11, Node 24.14.1, ffmpeg N-124716. No Docker daemon, no macOS host, no payment provider account.

---

## 1. Verdict

**Not ready to be marketed as a paid production product with full confidence.**

The software itself holds up. The test suite passes, the audit findings in `AUDIT_REPORT.md` are remediated, the live site is deployed and matches the repository, and the documentation now states its limits instead of overclaiming. That is the engineering side.

The commercial side is not shippable today:

1. Both checkout links are dead. `https://buy.polar.sh/polar_cl_8GNdKaiazCaCLqwFuDS5noS1SrupHxSK9nxr34Iohvq` and `https://buy.polar.sh/polar_cl_yJRWpGLnbQTA8xrpZcnxaqssxDdjegF3RGkea1ClCZG` both return `302` to `https://polar.sh/404`. Nobody can buy anything right now.
2. The price is contradictory across surfaces: euro subscriptions on the pricing page, dollar one-time prices on the landing page, the docs page, the EULA, the commercial licensing page and the Studio call to action.
3. The published release artifacts (`v0.2.7`, 2026-10-01) predate every audit remediation commit. The downloadable `.exe`, `.cjs`, Linux binary, macOS binary and ZIP are older code with the defects the audit fixed, including the broken license state in the standalone executable.
4. Current `main` CI is red. Six of nine matrix jobs fail at `Setup ffmpeg`; only the Windows jobs pass.
5. The macOS artifact was never executed by the project and is ad-hoc signed, so Gatekeeper blocks it on a default Mac. The Docker image was never built or published.

**Owner decision (2026-11-02): hold all marketing until the paid-production blockers are closed.** The capability could be launched as a beta with no payment flow, but that route is deliberately not being used. Do not announce paid availability until the blockers in section 4 are closed.

---

## 2. What was verified in this pass

| Check | Command or source | Result |
| :--- | :--- | :--- |
| Full test suite | `node --test` | **133 tests, 133 pass, 0 fail, 0 skip**, 76.9 s (re-run 2026-11-02; timing varies with load) |
| Site alias drift | `node scripts/sync-site.js --check` | pass, all five aliases byte-identical |
| Browser engine drift | `node scripts/build-engine.js --check` | pass, engine matches `src/` |
| Public routes | curl against `aureal.kellersystems.dev` | `/`, `/studio`, `/docs`, `/pricing`, `/verifier` all `200`; `/assets/aureal-engine.js` `200` |
| Error surface | curl on an unknown page and a missing asset | `/does-not-exist-abc123`, `/lexicon/test.html`, `/site/nope.html` and `/assets/nope.js` all return **`200 text/html`** with the landing page (about 100 KB); no `404.html` exists |
| Deployed content | byte comparison against git blobs | live engine is the same blob as `bcd7cc7` and current `HEAD`; HTML differs only by Cloudflare email protection and Rocket Loader injection |
| CI history | GitHub Actions API | last 3 runs on `main` failed; last green run 2026-10-20 14:09 UTC |
| Release artifacts | GitHub Releases API | latest is `v0.2.7` (2026-10-01), assets present, tag predates the fixes |
| Checkout links | curl | `302` to `polar.sh/404` for both product links |
| npm package | `registry.npmjs.org/aureal-watermark` | `404` (never published) |
| Repository visibility | GitHub API | public, 0 stars, SPDX license `NOASSERTION` (custom dual license) |
| Audit scripts | `audit/` | 15 measurement scripts present, not part of CI |

Repository state is clean apart from the untracked `SEO_AND_GEO_BLUEPRINT.md` and this report. The test suite and all remediation commits are on `origin/main`.

---

## 3. Launch gate matrix

| Gate | State | Evidence | What closes it |
| :--- | :--- | :--- | :--- |
| G1 Core DSP, WAV I/O, payload, CRC | PASS with documented limits | 133/133 tests; `AUDIT_REPORT.md` section 9; known limits published in README and `docs/` | Nothing |
| G2 Regression suite on the release commit | PASS locally, FAIL in CI | 133/133 locally; CI red on `main` | Green matrix (G3) |
| G3 CI matrix on GitHub runners | FAIL | 6 of 9 jobs fail at `Setup ffmpeg` on ubuntu and macOS | Repair the ffmpeg step, rerun matrix, record green run |
| G4 Release artifacts match reviewed code | FAIL | `v0.2.7` is 14 commits behind `HEAD` and contains pre-audit defects | Cut a new version, run the release workflow, smoke test the universal ZIP (D7) |
| G5 Checkout works end to end | FAIL | both Polar links 302 to `polar.sh/404` | Create products, update links, buy a test license, activate it in CLI and exe |
| G6 One price, one billing model, one currency | FAIL | pricing page `€89/yr` and `€290/yr`; landing and docs comparison tables `$249` and `$499` one-time; EULA and `COMMERCIAL_LICENSING.md` one-time dollar tiers; README euro monthly tiers; Studio CTA `$249` points at the `€89/yr` product | Subscription in EUR (D1); one price source propagated to all surfaces |
| G7 macOS artifact | FAIL | never run by the project; ad-hoc `codesign --sign -` only; unsigned downloads are quarantined by Gatekeeper | D3/D7: remove the macOS download and wording; ship the universal ZIP only |
| G8 Docker image | FAIL | image never built; nothing published to a registry; compose only builds locally | Build, run, verify health and one embed/detect round trip, publish |
| G9 REST service production posture | CONDITIONAL | RCE finding fixed, 32-case REST suite green, keep-alive fixed; still no authentication, no rate limit, CORS `*`, single-threaded synchronous DSP | Document the gateway requirement, add limits, or restrict the marketing claim to private networks |
| G10 Deployment process | CONDITIONAL | live content matches `HEAD`; no deploy workflow or recorded deploy commit; Cloudflare transforms page HTML at the edge | Record the deploy path and commit, verify routes after every deploy |
| G11 Legal pages and consumer terms | CONDITIONAL | Impressum (DDG section 5, Kleinunternehmer per section 19 UStG), privacy page, EULA and withdrawal text exist and are consistent with the privacy claims | Legal review; resolve `/legal/*` routing; align EULA tiers with the shop |
| G12 Claim accuracy | CONDITIONAL | disclaimers added across README, docs, site; three leftover claim strings remain (section 5) | Fix the strings in section 5 |
| G13 npm and JavaScript API distribution | FAIL if marketed as an API | README shows `import ... from "aureal-watermark"`; the package is not on npm | D4: remove the npm install/import instructions and point to the bundled `.cjs` |
| G14 License enforcement and support | CONDITIONAL | license state is a UI badge, not a technical gate; no support SLA or status page; purchase-to-activation path never tested with a live key | Decide enforcement position, document support, test a real purchase |
| G15 Version and release process | FAIL | code is 14 commits past the last tag, version still 0.2.7 | Bump version, tag, release, host the ZIP publicly, update the download links |
| G16 Crawl and error surface | CONDITIONAL | Live site: `/robots.txt`, `/sitemap.xml` and `/llms.txt` return `200` but serve the landing page; every unknown path, including a missing `/assets/*.js`, returns `200 text/html`. Locally built and verified but not deployed: `404.html`, `robots.txt`, `sitemap.xml`, `llms.txt`, `llms-full.txt`, `lexicon/` (30 pages + hub) | Deploy, disable the SPA fallback, verify `404` for unknown paths and `404` (not `200`) for missing assets, then submit the sitemap |

---

## 4. Blockers to close before paid production marketing

### P0-1 Restore the purchase path
Create the Polar products that the site advertises, replace both checkout URLs, and run one real purchase. Verify the full chain: checkout, license key delivery, `auralwatermark license --key ...`, license status in the CLI, and the stored license shown as active in the standalone executable and Desktop Studio. Evidence: order ID, activation transcript, screenshot of the license state.

### P0-2 One price story
**Decision D1: subscription in EUR.** One currency, one set of tier names. Update all of:

* `site/pricing.html` lines 721 to 828 (prices, buttons, toggle, comparison table)
* `site/index.html` lines 1205 to 1223 and `site/docs.html` lines 1205 to 1223 (the `$249` / `$499` comparison table)
* `site/studio.html` line 858 (the `$249` call to action on a product that is sold as `€89/yr`)
* `site/legal/terms.html` section 2 (tier grants and prices)
* `docs/COMMERCIAL_LICENSING.md` and the README pricing matrix
* the EULA tiers that are listed but not sold (`Voice AI $4,900/yr`, `Forensics $1,499`)

The perpetual `$249`/`$499` tiers in `site/legal/terms.html`, `docs/COMMERCIAL_LICENSING.md`, the EULA and the README must be converted to the EUR subscription prices or removed. The B2B tiers (`$2,900`, `$4,900`, `$1,499`) must become annual EUR subscriptions or be removed; no listed tier may be unbuyable.

Until then a buyer cannot tell what they are buying, and the contradiction is itself a misleading-pricing exposure in the EU.

### P0-3 Ship a release built from the fixed code
Bump the version, run the release workflow on a tag, and verify the chosen distribution (D7): the universal ZIP only, which contains `aureal-watermark.cjs`. Exercise the ZIP on Windows and Linux: `--help`, `gen`, `embed`, `detect`, `verify-report`, and the license display. Do not publish new `.exe`, Linux or macOS binaries. Remove the old binary download links and the "Windows, macOS, Linux" wording from the README and the site. Host the ZIP at a public URL that survives the private-repo migration, and update the README "latest" links. The currently published `v0.2.7` binaries must not be offered for download once the fixed release exists.

### P0-4 Green CI
The six failing jobs fail at `Setup ffmpeg` (ubuntu and macOS), not in the tests. Switch to per-OS ffmpeg installation (`apt-get`, `brew`, `choco`) or pin a working action revision, rerun the 3 by 3 matrix, and keep the run URL. Marketing that says "tested on macOS and Linux" needs a green CI run on macOS and Linux.

### P0-5 macOS honesty
**Resolved by D3/D7: the macOS executable is no longer distributed.** Remove the macOS download and the "Windows, macOS, Linux" wording from the README, the site and the release notes. The universal ZIP is the only download. If a native macOS binary is reintroduced later, build, run, sign and notarize it first: an ad-hoc signed binary will not open on a default Mac, and a paid customer hitting that is a refund and a support ticket.

### P0-6 Docker claim
Build and run `docker/docker-compose.yml`, run the health check and one embed/detect round trip, and publish the image or state clearly that users build it locally. Also document that the REST service has no authentication and belongs behind a private network or gateway. The README currently says "Deploy an air-gapped ... in 1 command" without a published image.

### P0-7 Fix the remaining overclaims
Three strings still conflict with the corrected documentation:

* `README.md` line 109: "Generate a cryptographically sealed forensic audit proof". The seal is a shared-secret HMAC and the default secret is public. Line 158 already says this correctly. Change line 109 to "tamper-evident HMAC-sealed report (shared secret, not a signature)".
* `docs/COMMERCIAL_LICENSING.md` line 23: the heading "EU AI Act Article 50 Machine-Readable Compliance" contradicts the disclaimer directly beneath it. Retitle it "EU AI Act Article 50 Context (No Compliance Certification)".
* `package.json` description, the meta description in `site/studio.html` and `site/verifier.html` (lines 8 and 10), the footer line in `site/verifier.html` line 706, and the `deepfake-detection` keyword all say or imply "AI detection". The product detects its own watermark; it does not detect AI-generated or deepfake audio. Replace with "audio watermarking for leak tracing and provenance records".

### P0-8 Remove the unpublished API instructions (D4)
`registry.npmjs.org/aureal-watermark` returns 404 while the README documents `import { embedWatermark } from "aureal-watermark"`. Do not publish the package now. Remove the npm install/import section and point developers to the `aureal-watermark.cjs` inside the universal ZIP, with a local-import example. If the package is revisited later, publish a pruned `dist/` with an `exports` map, not `src/`.

### P0-9 Legal and consumer review
Have a lawyer check the Impressum, privacy policy, EULA, withdrawal and refund text, auto-renewal disclosure, VAT treatment, and the regulatory wording in one pass. **Decision D5: split.** `kellersystems.dev` remains canonical for the Impressum and privacy page; the product EULA and terms ship with this repository. Update `_redirects` so `/impressum` and `/privacy` go to the portal and the product terms are served from this repo; delete or redirect the duplicate copies and use one label per destination. The Impressum has no telephone number; email is probably sufficient after the DDG change, but confirm it. The EULA lists tiers that are not for sale; D1 requires that list to match the shop.

### P0-10 Deployment record
There is no deploy workflow in the repository. Record how `aureal.kellersystems.dev` is deployed, from which commit, and verify the public routes after each deploy. The current live content does match `HEAD`, which is good; the missing piece is that nothing documents or enforces that.

---

## 5. Conditional findings (not launch blockers, fix soon)

* **Soft 404s across the whole site, including assets.** At audit time there was no `404.html` at the Pages output root, and the host answers every unmatched path with `index.html` and HTTP 200. The repository contains no catch-all `_redirects` rule, so the fallback is either a Pages SPA setting or an edge rule that is not in the repo. A root `404.html` and the crawl files now exist locally (G16), but none of it is deployed, so the live behaviour is unchanged. `/robots.txt`, `/sitemap.xml` and `/llms.txt` therefore *appear* to exist while serving the homepage, and a missing `/assets/*.js` returns 200 HTML instead of 404. Broken links, broken script paths and outages that only change the status code are invisible to monitoring, and search engines see duplicate or soft-404 pages. Fix: add a real `404.html` at the output root, disable the SPA fallback if it is enabled, then verify with `curl` that an unknown path returns 404 and a missing asset does not return HTML. Do this before submitting a sitemap or claiming any SEO work.
* **Windows and macOS binaries are unsigned — and, under D7, no longer distributed.** The universal ZIP contains JavaScript, so this stops being a shipping risk once the large binaries and their links are removed. If a native binary is reintroduced, sign it before offering it.
* **Seal default secret.** A report generated without `--key` is sealed with a public constant, so anyone can regenerate the seal. Add a `signature.keyProtection` field and print a warning in the default case, as already noted in `DEFERRED.md`.
* **REST service hardening.** No authentication, no rate limit, CORS `*`, and synchronous DSP on the event loop. Keep-alive was fixed; DoS resistance was not addressed. Document gateway deployment.
* **License enforcement.** The license is a badge, not a gate. That is a valid commercial choice; state it explicitly rather than implying enforcement.
* **Known weak cases are documented** and should stay in every marketing surface: sub-1 s excerpts never decode, 1.8 s crops recover about half the time, collusion and averaging defeat recovery, carrier band can be misreported after filtering, 8 kHz and broadband-noise hosts are weak.
* **Blind-mode BER is always 0** and the Studio still displays it. Hide it or label it "verify mode only".
* **`carrierBand` wire format** (F15 in `AUDIT_REPORT.md`) remains a documented limitation. Fixing it needs a versioned wire format.
* **npm package metadata.** `files` ships `site/` and `assets/`; with D4 (no npm publish) that is unused weight. If the package is revisited, prune it and publish `dist/` only.
* **No status page, no support SLA, no monitoring.** The commercial page promises priority support and same-day invoicing. Make sure those promises can be kept.

---

## 6. Marketing claim audit

### Claims that are supported by evidence now

* "Embeds a 32-bit keyed identifier using spread-spectrum audio watermarking."
* "Scanner recovers the identifier after selected MP3, AAC and Opus transcodes, resampling, crops, speed drift, stereo downmix, clipping, limiting and noise, on named fixtures. Recovery depends on source audio, codec and processing."
* "Batch distribution with a private recipient manifest and a leak audit tool that traces a recovered ID to a recipient."
* "Forensic report with SHA-256 and SHA-512 hashes, detection metrics and an HMAC seal covering the report body, verifiable with `verify-report` by anyone holding the key."
* "Runs locally with no telemetry; the only optional network request is license validation against Polar.sh."
* "Web Studio runs in the browser; the engine is generated from the same `src/` as the CLI and CI fails on drift."

### Claims that must not be made until the blocker is closed

* Any wording that presents a match as authorship, ownership, creation time, chain of custody or court evidence. The current docs already disclaim this; keep it that way in ads, landing pages, social posts and sales calls.
* "Cryptographically sealed forensic proof" without the shared-secret caveat (`README.md` line 109).
* "EU AI Act compliant" or "Article 50 compliance". The docs say "context, no certification". Keep that wording only.
* "Detects AI generated audio" or "deepfake detection". The product cannot do this (`package.json`, meta descriptions).
* "Collusion resistant" or "catches colluders". The README and licensing page already correct this; do not reintroduce it in the shop.
* "Deploy in 1 command" for Docker until the image exists and has been run.
* "Available for macOS" until the artifact is tested and signed.
* "Available on npm" until the package exists.
* "14-day money-back guarantee" claims tied to a checkout that 404s.

---

## 7. Can you market it now?

**Owner decision (2026-11-02): hold all marketing until paid-ready.** No beta or waitlist launch. The analysis below is kept as a fallback if that decision is reversed.

**As a paid production product: no.** The blockers above are real, and three of them (dead checkout, contradictory prices, stale binaries) would be visible to the first customer.

**If the hold is reversed, a beta with a waitlist would be possible today with a few copy fixes.** The site already carries the right disclaimers. That contingency would need:

1. Replace the two dead "Subscribe" buttons with "Join the early access list" or a mailto link, so no visitor can reach a 404 checkout.
2. Fix the three overclaim strings in P0-7.
3. State the limits from section 5 wherever the beta is described.
4. Add a real `404.html` and verify unknown paths return 404, so broken links and missing assets become visible before Search Console is pointed at the site.

That would be marketable as a beta, but it is not the chosen path. The audited engine, the published limitations and the local-only processing remain genuine differentiators, backed by 133 tests and the audit report.

**Full confidence requires** the ten P0 items closed with evidence. Engineering effort is roughly days, not weeks. The long pole is now the commercial and legal work: the pricing conversion, one live Polar product, one lawyer pass.

---

## 8. Sign-off checklist

Do not announce paid availability until every line is verifiable:

- [ ] CI green on all nine matrix jobs on the release commit (run URL recorded)
- [ ] Release workflow run on the new version tag succeeded (run URL recorded)
- [ ] Universal ZIP downloaded and exercised on Windows and Linux, including the `aureal-watermark.cjs` inside it
- [ ] No `.exe`/Linux/macOS download links remain; the "Windows, macOS, Linux" wording is gone (D7)
- [ ] Universal ZIP hosted at a public URL that survives the private-repo migration
- [ ] One real purchase completed: checkout, key delivery, CLI activation, CLI and Studio show active license
- [ ] Checkout links reachable and consistent with the prices on every page
- [ ] Subscription in EUR (D1) in all surfaces listed in P0-2
- [ ] Deployed site verified after the release: `/`, `/studio`, `/docs`, `/pricing`, `/verifier`, `/legal/*`, download links
- [ ] Unknown paths and missing assets return `404`, not `200` with the homepage
- [ ] `robots.txt`, `sitemap.xml` and `llms.txt` are real files with the correct content type (if SEO is in scope)
- [ ] `npm test`, `sync:site:check` and `build-engine --check` green on the release commit
- [ ] Docker image built, run, health check and embed/detect verified
- [ ] Claim sweep complete: section 6 strings fixed, disclaimers present on every marketing surface
- [ ] Legal review of Impressum, privacy, EULA, withdrawal, refund, auto-renewal and regulatory wording
- [ ] Support channel, refund process and invoicing process staffed
- [ ] No unsigned native binaries are offered; the macOS wording is gone (D3/D7)
- [ ] `DEFERRED.md` reviewed and either closed or consciously accepted for this launch

---

## 9. Owner decisions

These are the decisions that cannot be made correctly by engineering alone. Each one
changes copy, checkout, packaging or the release plan. The choices below were made on
2026-11-02; D6 remains open.

| ID | Decision | Taken 2026-11-02 | Blocks |
| :--- | :--- | :--- | :--- |
| D1 | Pricing model and currency | Subscription, EUR | P0-1, P0-2 |
| D2 | Launch route | Hold all marketing until paid-ready | Section 7 |
| D3 | macOS artifact | No macOS executable; universal ZIP only (via D7) | P0-5 |
| D4 | JavaScript API distribution | Remove the npm/import instructions | P0-8, G13 |
| D5 | Legal pages | Split: portal for Impressum/privacy, repo for product terms | P0-9, G11 |
| D6 | License enforcement | Open; recommended: badge, stated explicitly | G14 |
| D7 | Repository visibility and download hosting | Private repo, universal ZIP only | P0-3, P0-10 |
| D8 | SEO/GEO scope | Full Aureal lexicon now, built claim-safe and held until launch | G16, SEO plan |

### D1 Pricing model and currency

**Decision (2026-11-02): subscription in EUR.**

**Why:** The live pricing page already implements a monthly/yearly EUR toggle
(`€9`/`€29` monthly, `€89`/`€290` yearly) and the README matrix quotes the same tiers.
It is the only model already wired into the shop UI. Perpetual dollar tiers exist only in
the surrounding documents, so converting those is a copy task, not a new product build.

**What changes:** Make `site/pricing.html` the single price source and convert every other
surface to it: the `$249`/`$499` tables in `site/index.html` and `site/docs.html`, the
Studio CTA at `site/studio.html` line 858, `site/legal/terms.html` section 2, the EULA,
`docs/COMMERCIAL_LICENSING.md`, and the README matrix. Decide what happens to the B2B
tiers (`$2,900`, `$4,900`, `$1,499`): annual EUR subscriptions or removed. Do not leave a
tier listed that cannot be bought.

**Legal consequence:** subscription terms need auto-renewal, cancellation, withdrawal and
refund wording that the current perpetual EULA does not have. P0-9 must review that text.

### D2 Launch route

**Decision (2026-11-02): hold all marketing until paid-ready.**

**Why:** The first public impression should be a working paid product, not a beta. That
removes the waitlist and early-access route.

**What changes:** No public waitlist or "join the beta" page. The beta checklist in
section 7 is retained only as a fallback if the decision is reversed. The first
announcement coincides with the section 8 checklist. Time-to-launch is now dominated by
P0-1/P0-2 (pricing and checkout), P0-3 (release ZIP), P0-4 (CI) and P0-9 (legal).

### D3 macOS artifact

**Decision (2026-11-02): no macOS executable; universal ZIP only (via D7).**

**Why:** Gatekeeper blocks the current ad-hoc signed binary on a default Mac. Removing the
artifact eliminates the risk without buying an Apple Developer account or a Mac for a
download the product does not need.

**What changes:** Remove the macOS download and the "Windows, macOS, Linux" wording from
the README, the site, the release notes and the store copy. The ZIP contains
`aureal-watermark.cjs` and is cross-platform, but it requires Node.js 18+; document that
requirement honestly ("tested on Windows and Linux"). If a native macOS binary is ever
reintroduced, build, run, sign and notarize it first.

### D4 JavaScript API distribution

**Decision (2026-11-02): remove the npm install/import instructions.**

**Why:** `registry.npmjs.org/aureal-watermark` returns 404, so the documented import cannot
run. If the repository goes private (D7), publishing `src/` is also a licensing decision.
The universal ZIP already ships the self-contained `.cjs`, which is the supported local
entry point.

**What changes:** Delete the `npm install` and `import ... from "aureal-watermark"` section
from the README and replace it with a local-import example against the `.cjs` from the
ZIP. Keep `package.json` for the CLI, but stop advertising an npm API. If a package is
revisited later, publish a pruned `dist/` with an `exports` map, not `src/`.

### D5 Legal pages: one canonical home

**Decision (2026-11-02): split.** `kellersystems.dev` remains canonical for the Impressum
and privacy page; the product EULA and terms ship with this repository.

**Why:** Company-level pages belong to the company portal. Product-specific terms change
with the product version and must be reviewable next to the release. Today `_redirects`
sends `/legal/*` to the portal and drops the sub-path, so the in-repo copies are
unreachable and duplicative.

**What changes:** Update `_redirects` so `/impressum` and `/privacy` go to the portal and
the product terms are served from this repo (for example `/terms` → `site/legal/terms.html`).
Delete or redirect the duplicate copies, and use one label per destination: "Terms of
Service", "Commercial EULA" and "Terms of Sale" must not point at the same page.

### D6 License enforcement

**Status: open.** Recommended: (a) keep the license as a displayed badge and say so.

**Why:** The license state is already a UI badge, not a gate. That is a valid commercial
choice for a local tool, but it must be stated rather than implied. A real gate adds
offline-validation, support and refund complexity. Do not market "enforcement" until (b)
exists.

### D7 Repository visibility and download hosting

**Decision (2026-11-02): private repository, universal ZIP only.**

**Why:** The closed-source migration is planned, and publishing only the small ZIP avoids
hosting ~87 MiB Windows and ~118 MiB Linux executables on a private repo whose release
assets would require authentication. Cloudflare Pages can host the ~1 MB ZIP (25 MiB
per-file limit); the large binaries cannot be hosted there.

**What changes:**

* Make the download base configurable in one place, then point it at the public ZIP URL
  (Pages or a public bucket). Remove the `releases/latest/download/...` links before the
  repo goes private.
* Remove the `.exe`, Linux and macOS download links and the "Windows, macOS, Linux"
  wording from the README, site footers, docs and release notes.
* Repoint `package.json` `repository`, `homepage` and `bugs`, and every GitHub blob link,
  at the website or remove them.
* Consequence to document: buyers need Node.js 18+ to run `node aureal-watermark.cjs`.
  The quickstart must lead with that requirement instead of "download the .exe".
* Do the private migration last, after the ZIP is public and the links are verified.

### D8 SEO/GEO scope

**Decision (2026-11-02): full Aureal lexicon now, built claim-safe and held until launch.**

**Why:** The product has enough technical substance for 20–40 high-intent pages (codec
survival, crops, collusion, HMAC seals, leak tracing, AI Act context). Building the
content now uses the audit evidence while it is fresh; holding publication respects D2.

**What changes:**

* Ship the technical foundations first: a real `404.html`, `robots.txt`, `sitemap.xml`,
  `llms.txt` and `llms-full.txt`. Do not submit a sitemap while unknown paths return 200.
* Configure the blueprint for `aureal.kellersystems.dev` (`outDir: lexicon`, CTA `/studio`,
  the claim limits from section 5). The blueprint now carries an Aureal topic matrix.
  All 30 pages generated on 2026-11-02 in `lexicon/` (the full matrix: Basics,
  Robustness, Forensics, Workflow, Compliance and the voice-provider use case), plus the
  hub, `404.html`, `robots.txt`, `sitemap.xml`, `llms.txt` and `llms-full.txt`.
  `npm run seo` builds them; `npm run seo:check` fails on drift. Canonicals, sitemap and
  internal links use the extensionless URL because Cloudflare Pages 308-redirects every
  `.html` path (verified on `/site/legal/terms.html`). Nothing is deployed.
* Write 20–40 pages with claim-safe copy: no "AI detection", no "forensic proof", no
  "collusion resistant", no "EU AI Act compliant". Every page carries the known limits
  that apply to it and the "match is not authorship" disclaimer.
* Build now, publish with the paid launch. A lexicon is marketing; D2 controls when it
  goes public.

---

## 10. Suggested order

1. **Lock D1 and D2.** D1 is decided (subscription, EUR); rewrite P0-2 now because it is
   the input to P0-1. Start P0-9 (lawyer) in parallel; it is the long pole.
2. **Fix CI (P0-4) and add `404.html` (G16) in parallel.** Both are independent of the
   commercial work and cheap.
3. **Run the P0-7 claim sweep before anything is published.** The full Aureal lexicon
   (D8) can be written alongside, but every page inherits these claim rules.
4. **Cut the release (P0-3) as the universal ZIP only (D7).** Host the ZIP publicly,
   remove the `.exe`/Linux/macOS links and wording, and verify the ZIP on Windows and
   Linux. Shipping binaries before the claim fixes puts the old overclaims into the
   download; going private before the ZIP is hosted breaks every download link.
5. **Close P0-1/P0-2, P0-5/P0-6/P0-8/P0-9/P0-10**, then run the section 8 checklist.
6. **Publish the lexicon with the paid launch (D2 + D8).** Submit the sitemap only after
   unknown paths return 404 and the claim sweep is complete.
7. **Go private last.** Repoint the download base, README links, footers and
   `package.json` metadata, then re-run the section 8 deployment checks.

Parallelizable: CI repair, `404.html`, the lexicon content, removing the npm instructions,
and the legal review request. Sequential: pricing copy → Polar products → checkout test →
release ZIP → public launch.

---

## Appendix A: Commands used

```bash
node --test
node scripts/sync-site.js --check
node scripts/build-engine.js --check
curl -sI https://aureal.kellersystems.dev/<route>
curl -s -o /dev/null -w "%{http_code} %{content_type}\n" https://aureal.kellersystems.dev/does-not-exist-abc123
curl -s -o /dev/null -w "%{http_code} %{content_type}\n" https://aureal.kellersystems.dev/assets/does-not-exist.js
curl -s -o /dev/null -w "%{http_code} %{redirect_url}" https://buy.polar.sh/<link>
curl -s https://api.github.com/repos/KELLERBABG/Aureal-Watermark/actions/workflows/test.yml/runs?per_page=5
curl -s https://api.github.com/repos/KELLERBABG/Aureal-Watermark/releases/latest
curl -s -o /dev/null -w "%{http_code}" https://registry.npmjs.org/aureal-watermark
```

## Appendix B: Source documents

* `AUDIT_REPORT.md` (audit evidence and remediation log, F1 to F31)
* `DEFERRED.md` (open owner decisions: closed-source migration, pricing, legal routing, seal secret). Its status paragraph still says 123 tests; the 133-test run above supersedes it.
* `SEO_AND_GEO_BLUEPRINT.md` (the SEO/GEO playbook; executed for the first batch: generator, hub, `404.html`, `robots.txt`, `sitemap.xml`, `llms.txt` and `llms-full.txt`. Deployment is open)
* `docs/RELEASE_READINESS.md` (earlier readiness review)
* `README.md`, `docs/COMMERCIAL_LICENSING.md`, `site/legal/terms.html`, `site/legal/impressum.html`, `site/legal/privacy.html`
