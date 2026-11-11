# ClauseLens Stress-Korpus — Detektionsbericht

Quelle: test/stress/corpus/*.js · Engine: src/analyze.js (47 Detektoren)

Falle gesamt: **380** · bestanden: **378** · Fehlschläge: **2** (99.5% bestanden)

## Aggregate

| Metrik | Wert |
|---|---|
| Täuschende Klauseln | 290 |
| davon Zieldetektor getroffen | 288 (99.3%) |
| davon ausreichend scharf (≥ erwartet) | 288 (99.3%) |
| stille Blindspots (kein einziger Treffer) | 2 |
| nur Fremdtreffer (falscher Detektor) | 0 |
| harmlose Klauseln | 90 |
| davon sauber (kein mittel/hoch) | 90 (100%) |
| False-Positive-Fälle | 0 |
| Engine-Fehler | 0 |

## Trefferquote je Detektor (nur täuschende Fälle)

| Detektor | Fälle | bestanden | fehlend | Fehlschläge |
|---|---|---|---|---|
| `deemed_acceptance` | 12 | 11 | 1 | ingest-glued-02 |
| `liability` | 17 | 16 | 1 | ingest-column-01 |
| `agency_commission_exclusion` | 1 | 1 | 0 |  |
| `agency_goodwill_waiver` | 1 | 1 | 0 |  |
| `as_is_software_disclaimer` | 6 | 6 | 0 |  |
| `assignment_restriction` | 4 | 4 | 0 |  |
| `asymmetric_confidentiality` | 4 | 4 | 0 |  |
| `asymmetric_termination` | 6 | 6 | 0 |  |
| `audit_log_manipulation` | 5 | 5 | 0 |  |
| `auto_renewal` | 9 | 9 | 0 |  |
| `burden_of_proof` | 8 | 8 | 0 |  |
| `data_grant` | 9 | 9 | 0 |  |
| `data_lockin` | 5 | 5 | 0 |  |
| `default_interest` | 12 | 12 | 0 |  |
| `dynamic_url_referral` | 6 | 6 | 0 |  |
| `early_termination_charge` | 1 | 1 | 0 |  |
| `entire_agreement` | 6 | 6 | 0 |  |
| `excessive_survival` | 6 | 6 | 0 |  |
| `extended_retention_of_title` | 7 | 7 | 0 |  |
| `feedback_ip_grab` | 4 | 4 | 0 |  |
| `fictitious_receipt` | 5 | 5 | 0 |  |
| `force_majeure_asymmetry` | 5 | 5 | 0 |  |
| `insolvency_termination` | 5 | 5 | 0 |  |
| `international_data_processing` | 1 | 1 | 0 |  |
| `jurisdiction` | 14 | 14 | 0 |  |
| `maintenance_conditional_shutdown` | 1 | 1 | 0 |  |
| `mandatory_expert_determination` | 1 | 1 | 0 |  |
| `marketing_reference` | 5 | 5 | 0 |  |
| `non_compete` | 6 | 6 | 0 |  |
| `non_solicitation` | 6 | 6 | 0 |  |
| `notice_period` | 7 | 7 | 0 |  |
| `penalty` | 12 | 12 | 0 |  |
| `price_escalation` | 10 | 10 | 0 |  |
| `salvatorische_klausel_falle` | 5 | 5 | 0 |  |
| `service_credit_forfeiture` | 1 | 1 | 0 |  |
| `service_suspension` | 6 | 6 | 0 |  |
| `setoff_restriction` | 9 | 9 | 0 |  |
| `statute_of_limitations` | 8 | 8 | 0 |  |
| `subcontractor_uncontrolled` | 6 | 6 | 0 |  |
| `tax_indemnity_shift` | 8 | 8 | 0 |  |
| `third_party_indemnity` | 5 | 5 | 0 |  |
| `unilateral_change` | 7 | 7 | 0 |  |
| `unilateral_convenience_termination` | 4 | 4 | 0 |  |
| `unreasonable_audit` | 4 | 4 | 0 |  |
| `unverbindliche_sla_falle` | 7 | 7 | 0 |  |
| `vicarious_agents` | 5 | 5 | 0 |  |
| `written_form_trap` | 8 | 8 | 0 |  |

## Trefferquote je Evasionstechnik (nur täuschende Fälle)

| Technik | Fälle | bestanden | fehlend | Bestehensquote |
|---|---|---|---|---|
| `typo-evasion` | 52 | 50 | 2 | 96.2% |
| `compound-multi` | 4 | 4 | 0 | 100% |
| `cross-reference` | 12 | 12 | 0 | 100% |
| `dach` | 4 | 4 | 0 | 100% |
| `direct` | 40 | 40 | 0 | 100% |
| `english` | 25 | 25 | 0 | 100% |
| `euphemism` | 42 | 42 | 0 | 100% |
| `full-contract` | 15 | 15 | 0 | 100% |
| `list-buried` | 10 | 10 | 0 | 100% |
| `nominal-style` | 21 | 21 | 0 | 100% |
| `split-sentence` | 12 | 12 | 0 | 100% |
| `synonym` | 38 | 38 | 0 | 100% |
| `word-number` | 15 | 15 | 0 | 100% |

### Kontrollgruppe: unverschleierte Klauseln (technique `direct`)

Von 40 im Klartext formulierten Fallen wurden 40 erkannt (100%). Diese Quote muss nahe 100 % liegen — Verluste hier sind reine Regellücken, keine Formulierungsblindspots.

## Harmlose Klauseln je Themenbereich

| Bereich | Fälle | sauber | False Positives |
|---|---|---|---|
| `agency_commission_exclusion` | 2 | 2 | 0 |
| `agency_goodwill_waiver` | 1 | 1 | 0 |
| `as_is_software_disclaimer` | 1 | 1 | 0 |
| `assignment_restriction` | 2 | 2 | 0 |
| `asymmetric_confidentiality` | 2 | 2 | 0 |
| `asymmetric_termination` | 1 | 1 | 0 |
| `auto_renewal` | 2 | 2 | 0 |
| `burden_of_proof` | 2 | 2 | 0 |
| `data_grant` | 2 | 2 | 0 |
| `data_lockin` | 2 | 2 | 0 |
| `deemed_acceptance` | 3 | 3 | 0 |
| `default_interest` | 3 | 3 | 0 |
| `dynamic_url_referral` | 2 | 2 | 0 |
| `early_termination_charge` | 1 | 1 | 0 |
| `entire_agreement` | 3 | 3 | 0 |
| `excessive_survival` | 2 | 2 | 0 |
| `extended_retention_of_title` | 1 | 1 | 0 |
| `feedback_ip_grab` | 1 | 1 | 0 |
| `fictitious_receipt` | 1 | 1 | 0 |
| `force_majeure_asymmetry` | 2 | 2 | 0 |
| `insolvency_termination` | 1 | 1 | 0 |
| `international_data_processing` | 1 | 1 | 0 |
| `jurisdiction` | 4 | 4 | 0 |
| `liability` | 3 | 3 | 0 |
| `maintenance_conditional_shutdown` | 1 | 1 | 0 |
| `mandatory_expert_determination` | 1 | 1 | 0 |
| `marketing_reference` | 2 | 2 | 0 |
| `mixed` | 10 | 10 | 0 |
| `non_compete` | 2 | 2 | 0 |
| `non_solicitation` | 1 | 1 | 0 |
| `notice_period` | 4 | 4 | 0 |
| `penalty` | 2 | 2 | 0 |
| `price_escalation` | 3 | 3 | 0 |
| `salvatorische_klausel_falle` | 1 | 1 | 0 |
| `service_credit_forfeiture` | 1 | 1 | 0 |
| `setoff_restriction` | 2 | 2 | 0 |
| `statute_of_limitations` | 2 | 2 | 0 |
| `subcontractor_uncontrolled` | 3 | 3 | 0 |
| `tax_indemnity_shift` | 2 | 2 | 0 |
| `unilateral_change` | 1 | 1 | 0 |
| `unilateral_convenience_termination` | 1 | 1 | 0 |
| `unreasonable_audit` | 1 | 1 | 0 |
| `unverbindliche_sla_falle` | 2 | 2 | 0 |
| `written_form_trap` | 3 | 3 | 0 |

## Blindspots (2)

### ingest-glued-02 — `deemed_acceptance` (typo-evasion)

- Erwartet: hoch
- Befund: Zieldetektor nicht getroffen
- Treffer: keine
- Risiko-Score: 0 (niedrig)
- Lücke: Verklebte Extraktion der Rügeklausel. Unbeschädigt: 'Erkennbare Mängel sind binnen einer Ausschlussfrist von 48 Stunden nach Ablieferung schriftlich zu rügen; widrigenfalls gilt die Ware als vorbehaltlos genehmigt.' Das Semikolon trennt die Sätze wie gewohnt, aber 'Ausschlussfrist von 48 Stunden' und 'widrigenfalls gilt ... genehmigt' brauchen Leerzeichen.

```text
ErkennbareMängelsindbinneneinerAusschlussfristvon48StundennachAblieferungschriftlichzurügen;widrigenfallsgiltdieWarealsvorbehaltlosgenehmigt.
```

### ingest-column-01 — `liability` (typo-evasion)

- Erwartet: hoch
- Befund: Zieldetektor nicht getroffen
- Treffer: keine
- Risiko-Score: 0 (niedrig)
- Lücke: Zweispaltiges PDF, die Extraktion liest beide Spalten zeilenweise durchmischt. Unbeschädigt: 'Die Haftung des Anbieters für sämtliche Schäden, gleich aus welchem Rechtsgrund, wird hiermit vollumfänglich ausgeschlossen.' Subjekt und Rechtsfolge landen in getrennten Sätzen, die Fremdsätze enthalten keinen Haftungsbezug.

```text
Die Haftung des Anbieters
Die Parteien benennen je einen Projektleiter.
für sämtliche Schäden, gleich aus welchem Rechtsgrund,
Die Projektleiter stimmen sich wöchentlich ab.
wird hiermit vollumfänglich ausgeschlossen.
```

## False Positives auf harmlosen Klauseln (0)

_Keine._
