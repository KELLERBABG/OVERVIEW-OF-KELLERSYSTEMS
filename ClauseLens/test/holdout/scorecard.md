# Held-out Scorecard

Evaluation corpus: generated contracts plus published clause wordings. A trap counts only when a finding
of the expected detector type overlaps its marked lines. The eight formerly uncovered clauses were used to design these new detectors, so this post-change score is an iteration result—not an independent holdout estimate; see scorecard-baseline.md for the pre-change score.

| Korpus | Herkunft | Befunde | Score | erkannt | Trefferquote | Fehlalarme (harmlos) | ungedeckt |
|---|---|---|---|---|---|---|---|
| adv-cloud-plattform | generated | 10 | 100 | 9/9 | 100% | 0/10 | 0 |
| adv-it-wartungsvertrag | generated | 6 | 74 | 6/6 | 100% | 0/6 | 0 |
| adv-marketing-agentur | generated | 6 | 66 | 5/5 | 100% | 0/6 | 0 |
| adv-personalberatung | generated | 8 | 100 | 7/7 | 100% | 0/6 | 0 |
| adv-spedition-kontraktlogistik | generated | 6 | 82 | 6/6 | 100% | 0/7 | 0 |
| adv-zuliefervertrag | generated | 6 | 82 | 6/6 | 100% | 0/8 | 0 |
| gen-franchise-handelsvertreter | generated | 13 | 100 | 14/14 | 100% | 0/6 | 0 |
| gen-saas-subscription | generated | 16 | 100 | 16/16 | 100% | 0/7 | 0 |
| gen-werkvertrag-systemhaus | generated | 26 | 100 | 26/26 | 100% | 0/6 | 0 |
| web-real-clauses | web | 20 | 100 | 14/14 | 100% | 0/1 | 0 |
| **Summe** | | | | **109/109** | **100%** | **0/63** | **0** |

## Trefferquote je Detektor (schlechteste zuerst)

| Detektor | erkannt | Quote |
|---|---|---|
| price_escalation | 9/9 | 100% |
| liability | 7/7 | 100% |
| penalty | 6/6 | 100% |
| jurisdiction | 5/5 | 100% |
| non_solicitation | 5/5 | 100% |
| unilateral_change | 5/5 | 100% |
| data_grant | 4/4 | 100% |
| deemed_acceptance | 4/4 | 100% |
| written_form_trap | 4/4 | 100% |
| as_is_software_disclaimer | 4/4 | 100% |
| auto_renewal | 4/4 | 100% |
| setoff_restriction | 3/3 | 100% |
| service_suspension | 3/3 | 100% |
| notice_period | 3/3 | 100% |
| third_party_indemnity | 3/3 | 100% |
| unilateral_convenience_termination | 2/2 | 100% |
| data_lockin | 2/2 | 100% |
| unverbindliche_sla_falle | 2/2 | 100% |
| entire_agreement | 2/2 | 100% |
| dynamic_url_referral | 2/2 | 100% |
| fictitious_receipt | 2/2 | 100% |
| unreasonable_audit | 2/2 | 100% |
| feedback_ip_grab | 2/2 | 100% |
| marketing_reference | 2/2 | 100% |
| excessive_survival | 2/2 | 100% |
| burden_of_proof | 2/2 | 100% |
| non_compete | 2/2 | 100% |
| international_data_processing | 2/2 | 100% |
| tax_indemnity_shift | 1/1 | 100% |
| extended_retention_of_title | 1/1 | 100% |
| assignment_restriction | 1/1 | 100% |
| agency_commission_exclusion | 1/1 | 100% |
| agency_goodwill_waiver | 1/1 | 100% |
| asymmetric_termination | 1/1 | 100% |
| salvatorische_klausel_falle | 1/1 | 100% |
| mandatory_expert_determination | 1/1 | 100% |
| service_credit_forfeiture | 1/1 | 100% |
| statute_of_limitations | 1/1 | 100% |
| subcontractor_uncontrolled | 1/1 | 100% |
| insolvency_termination | 1/1 | 100% |
| early_termination_charge | 1/1 | 100% |
| maintenance_conditional_shutdown | 1/1 | 100% |

## adv-cloud-plattform (B2B Cloud-Plattform (SaaS, deutsches Recht, deutschsprachig))


## adv-it-wartungsvertrag (B2B IT-Wartungsvertrag (deutsches Recht, deutschsprachig))


## adv-marketing-agentur (B2B Marketingagentur (deutsches Recht, deutschsprachig))


## adv-personalberatung (B2B Personalberatung/Vermittlung (deutsches Recht, deutschsprachig))


## adv-spedition-kontraktlogistik (B2B Kontraktlogistik (deutsches Recht, deutschsprachig))


## adv-zuliefervertrag (B2B Rahmenliefervertrag Technikkomponenten (deutsches Recht, deutschsprachig))


## gen-franchise-handelsvertreter (Franchise- und Handelsvertretervertrag (Systemvertrieb, deutsche Systemzentrale, Systempartner als Unternehmer))


## gen-saas-subscription (B2B SaaS/Cloud-Subscription (österreichisches Recht, deutschsprachig))


## gen-werkvertrag-systemhaus (IT-Systemhaus Werk-/Dienstvertrag (Anlagensteuerung, B2B, Sitz Schweiz))


## web-real-clauses (Real published clause wordings from provider terms and court decisions (SaaS, cloud, energy, telecom, insurance))

