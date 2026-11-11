# Corpus Specification (read this before writing anything)

Goal: adversarially pentest the ClauseLens rule engine (`src/analyze.js`). Every contract you
write is fed to `analyzeContract(text)`. The measurement is: did a finding of the expected
detector type cover the marked line range, and did any finding wrongly cover a benign clause.

## Layout

- Contracts: `Verträge/corpus/<slug>.txt`
- Labels:    `Verträge/corpus/labels/<slug>.json`   (exact basename, `.json`)
- Public baseline: `Verträge/public/<slug>.txt` + `Verträge/public/labels/<slug>.json`

Your assigned prefix is in your task. Every slug you write MUST start with that prefix.
Never touch files outside your prefix. Never edit `src/**`, `Verträge/tools/**`, `Verträge/README.md`,
`Verträge/CONTRACT_SPEC.md`, or another prefix.

## Contract text rules

- German legal prose (B2B), realistic register and layout: party block, `§ 1 Gegenstand`, numbered
  paragraphs, definitions, annex references, signature block.
- Length 60 to 160 lines. Plain UTF-8, `.txt`, line breaks only. No markdown, no tables.
- One clause per paragraph, separated by a blank line. The engine groups findings per paragraph,
  so one trap per paragraph.
- No em dashes (—) and no en dashes (–) anywhere. Use commas, colons, periods.
- Do not copy verbatim from the existing holdout corpus in `test/holdout/contracts/`. Read it to
  learn what the engine already catches, then write clauses that are structurally different.

## Label schema

```json
{
  "slug": "saas-014",
  "source": "generated",
  "domain": "B2B SaaS, deutsches Recht",
  "traps": [
    { "marker": "setzt sich die Laufzeit um weitere zweiundsiebzig Monate fort",
      "expectedType": "auto_renewal", "note": "why this is a trap and which register wording was avoided" }
  ],
  "benign": [
    { "marker": "Die Haftung für Vorsatz und grobe Fahrlässigkeit bleibt unberührt",
      "note": "conforming clause, must NOT produce any finding" }
  ],
  "uncovered": [
    { "marker": "...", "note": "deceptive clause that no detector type describes" }
  ]
}
```

- `marker` MUST be a verbatim substring of the contract, inside a single paragraph, 25 to 100
  characters, and MUST occur exactly once in the file. Copy and paste it from the text you wrote.
- `expectedType` MUST be one of the 47 detector types listed below. A trap whose type is not a
  detector belongs in `uncovered`, not in `traps`.
- `traps` >= 6, `benign` >= 5 per contract. `uncovered` is optional but wanted.
- `benign` clauses are the false positive controls. Make them hard: correct limitation of
  liability, mutual confidentiality, 14 day subcontractor notice, 30 day payment term, free data
  export, text form allowed, mutual termination rights.

## Adversarial requirement (this is the point)

The engine is measured on generalization, not on register words. For every contract, at least
60 percent of traps must AVOID the literal phrasing that the detectors were built around. Use:

- paraphrase instead of register ("Vorlaufzeit" for "Kündigungsfrist", "klaglos stellen" for "freistellen")
- negation that reads as permission, or permission that reads as a restriction
- a trap split across two paragraphs or referenced by section number
- alternative spellings, compounds, English clauses, footnotes, annexes
- numbers as words, unusual but valid units ("zweiundsiebzig Monate", "anderthalb Jahre")
- obfuscation: NBSP (\u00a0), soft hyphen (\u00ad), zero width space (\u200b), homoglyphs (Cyrillic а, е, о)
  used inside a trap phrase. Mark the obfuscated substring as the marker only if the obfuscation
  is preserved in the JSON string; prefer a clean sub-phrase as marker for obfuscated traps.

Vary style per file so the corpus is not 300 copies of one template. Vary parties, seat, law
(DE / AT / CH / EU), industry, term, and clause order.

## Domain coverage per slice

Write contracts in the domain assigned to your prefix. Cover many sub-domains inside it, and
spread coverage across all 47 detectors. Report your per-detector counts at the end.

## Detector types (expectedType values)

auto_renewal, notice_period, liability, penalty, jurisdiction, non_compete, data_grant,
default_interest, unilateral_change, setoff_restriction, deemed_acceptance, statute_of_limitations,
written_form_trap, burden_of_proof, dynamic_url_referral, vicarious_agents, unreasonable_audit,
data_lockin, third_party_indemnity, asymmetric_termination, assignment_restriction,
force_majeure_asymmetry, non_solicitation, unilateral_convenience_termination, excessive_survival,
price_escalation, salvatorische_klausel_falle, extended_retention_of_title, subcontractor_uncontrolled,
audit_log_manipulation, service_suspension, insolvency_termination, tax_indemnity_shift,
asymmetric_confidentiality, as_is_software_disclaimer, entire_agreement, marketing_reference,
feedback_ip_grab, fictitious_receipt, unverbindliche_sla_falle, agency_commission_exclusion,
agency_goodwill_waiver, international_data_processing, mandatory_expert_determination,
service_credit_forfeiture, early_termination_charge, maintenance_conditional_shutdown

## Verification before you finish

Run the validator on your own files:

```
node "Verträge/tools/validate.mjs" <your-prefix>
```

It must print `OK` for every file. Fix every reported issue: unresolved marker, duplicate marker,
unknown detector type, missing label, slug/prefix mismatch. Do not report success while the
validator reports an issue.
