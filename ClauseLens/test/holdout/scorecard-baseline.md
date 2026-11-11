# Held-out Scorecard

Contracts the rules were never written against. A trap counts as detected only when a finding
of the expected detector type overlaps the planted clause's lines.

| Korpus | Herkunft | Befunde | Score | erkannt | Trefferquote | Fehlalarme (harmlos) | ungedeckt |
|---|---|---|---|---|---|---|---|
| gen-franchise-handelsvertreter | generated | 1 | 15 | 1/12 | 8.3% | 0/6 | 2 |
| gen-saas-subscription | generated | 3 | 45 | 3/13 | 23.1% | 0/7 | 3 |
| gen-werkvertrag-systemhaus | generated | 2 | 14 | 1/23 | 4.3% | 0/6 | 3 |
| web-real-clauses | web | 5 | 75 | 3/14 | 21.4% | 0/1 | 1 |
| **Summe** | | | | **8/62** | **12.9%** | **0/20** | **9** |

## Trefferquote je Detektor (schlechteste zuerst)

| Detektor | erkannt | Quote |
|---|---|---|
| price_escalation | 0/7 | 0% |
| unilateral_change | 0/5 | 0% |
| penalty | 0/4 | 0% |
| notice_period | 0/3 | 0% |
| third_party_indemnity | 0/3 | 0% |
| non_solicitation | 0/2 | 0% |
| data_grant | 0/2 | 0% |
| setoff_restriction | 0/2 | 0% |
| service_suspension | 0/2 | 0% |
| data_lockin | 0/1 | 0% |
| asymmetric_termination | 0/1 | 0% |
| unverbindliche_sla_falle | 0/1 | 0% |
| deemed_acceptance | 0/1 | 0% |
| dynamic_url_referral | 0/1 | 0% |
| statute_of_limitations | 0/1 | 0% |
| burden_of_proof | 0/1 | 0% |
| feedback_ip_grab | 0/1 | 0% |
| marketing_reference | 0/1 | 0% |
| fictitious_receipt | 0/1 | 0% |
| subcontractor_uncontrolled | 0/1 | 0% |
| insolvency_termination | 0/1 | 0% |
| excessive_survival | 0/1 | 0% |
| written_form_trap | 0/1 | 0% |
| liability | 2/6 | 33% |
| as_is_software_disclaimer | 1/3 | 33% |
| auto_renewal | 2/4 | 50% |
| non_compete | 1/2 | 50% |
| jurisdiction | 1/2 | 50% |
| salvatorische_klausel_falle | 1/1 | 100% |

## gen-franchise-handelsvertreter (Franchise- und Handelsvertretervertrag (Systemvertrieb, deutsche Systemzentrale, Systempartner als Unternehmer))


Nicht erkannt:

- unilateral_change @L21 :: Ein Anspruch auf Beibehaltung der bisherigen Abgrenzung besteht nicht
- unilateral_change @L24 :: hält es laufend aktuell und teilt Änderungen in Textform mit
- unilateral_change @L30 :: Der Systempartner nimmt die Festsetzung mit Aufnahme der Tätigkeit im 
- non_compete @L64 :: betreibt für die Dauer von zwei Jahren nach dem Ende der Zusammenarbei
- penalty @L55 :: Sie bleibt unabhängig davon geschuldet, ob der Systemzentrale ein Scha
- penalty @L36 :: stellt die Systemzentrale den Fehlbetrag in Höhe von sechs Prozent als
- data_lockin @L68 :: verlangt die Systemzentrale eine Bearbeitungspauschale von 1.500 EUR j
- auto_renewal @L60 :: setzt sich nach Ablauf der Festlaufzeit jeweils um ein weiteres Jahr f
- notice_period @L60 :: achtzehn Monate vor dem jeweiligen Periodenende in Textform beendet wi
- liability @L75 :: Für leicht fahrlässig verursachte Vermögensschäden wird nicht eingesta
- non_solicitation @L71 :: Personen, die für die jeweils andere Seite tätig waren, mit dem Ziel e

## gen-saas-subscription (B2B SaaS/Cloud-Subscription (österreichisches Recht, deutschsprachig))


Nicht erkannt:

- auto_renewal @L48 :: setzt sich nach Ablauf der Anfangsperiode jeweils um zwölf Monate fort
- notice_period @L50 :: unter Einhaltung einer Vorlaufzeit von neun Monaten zulässig
- price_escalation @L38 :: an die Preissteigerung angepasst
- unilateral_change @L40 :: Erhebt der Kunde nicht binnen zehn Tagen Einspruch
- liability @L68 :: sieht der Kunde wegen Störungen des Geschäftsbetriebs von Ansprüchen a
- third_party_indemnity @L74 :: hat der Kunde den Anbieter insoweit klaglos zu stellen
- as_is_software_disclaimer @L58 :: wird keine Zusage übernommen
- data_grant @L80 :: darf der Anbieter in eigener Verantwortung verwerten
- setoff_restriction @L70 :: nur nach vorheriger schriftlicher Bestätigung durch den Anbieter in An
- asymmetric_termination @L52 :: Ein Anspruch auf Loslösung vor Ablauf der Periode besteht nur

## gen-werkvertrag-systemhaus (IT-Systemhaus Werk-/Dienstvertrag (Anlagensteuerung, B2B, Sitz Schweiz))


Nicht erkannt:

- unverbindliche_sla_falle @L16 :: Ansprüche wegen einer Unterschreitung dieser Verfügbarkeit bestehen ni
- penalty @L19 :: Der Nachweis eines geringeren Schadens ist nicht zulässig
- deemed_acceptance @L22 :: wird die Teilleistung als vertragsgemäß behandelt
- price_escalation @L25 :: an die Entwicklung des Landesindexes der Konsumentenpreise angepasst
- dynamic_url_referral @L28 :: abrufbar unter www.systemhaus-beispiel.ch/bedingungen
- as_is_software_disclaimer @L30 :: keine Gewähr dafür, dass die Steuerung sämtliche Störungen erkennt
- statute_of_limitations @L31 :: erlöschen neun Monate nach der Inbetriebnahme
- burden_of_proof @L33 :: ausschließlich der Besteller darlegungs- und beweisbelastet
- data_grant @L37 :: zeitlich unbeschränkt und ohne Vergütung für eigene Zwecke nutzen
- feedback_ip_grab @L37 :: in seine Produkte einfließen lassen
- non_solicitation @L40 :: keine Fachkräfte abwerben, die für den Auftragnehmer tätig sind
- marketing_reference @L42 :: den Besteller als Kunden nennen und dessen Zeichen verwenden
- fictitious_receipt @L43 :: gelten als zugegangen, sobald sie in das für die Partei hinterlegte el
- subcontractor_uncontrolled @L45 :: Drittanbieter heranziehen, ohne dies dem Besteller vorher anzuzeigen  [stattdessen: liability]
- third_party_indemnity @L46 :: unabhängig davon, ob den Besteller ein Verschulden trifft  [stattdessen: liability]
- liability @L48 :: nur bis zur Höhe desjenigen Entgelts
- setoff_restriction @L49 :: Einbehalte des Bestellers sind nur mit vorheriger Zustimmung des Auftr
- service_suspension @L51 :: unterbricht der Auftragnehmer die Leistungserbringung ohne vorherige A
- insolvency_termination @L52 :: endet der Vertrag ohne weiteres
- excessive_survival @L53 :: sondern besteht auf Dauer
- jurisdiction @L54 :: ausschließlich die Gerichte am Geschäftssitz des Auftragnehmers berufe
- written_form_trap @L59 :: bedürfen einer von beiden Parteien unterzeichneten und notariell begla

## web-real-clauses (Real published clause wordings from provider terms and court decisions (SaaS, cloud, energy, telecom, insurance))


Nicht erkannt:

- notice_period @L54 :: die mit der Freischaltung des DGN-Anschlusses zu laufen beginnen soll
- price_escalation @L10 :: zum Anfang einer jeden Laufzeitverlängerung, den Preis für die jeweili
- price_escalation @L14 :: Google may change the Fees at any time
- service_suspension @L18 :: Google may immediately Suspend all or part of Customer's use of the Se
- liability @L26 :: MAXIMUM LIABILITY FOR DAMAGES SHALL BE LIMITED TO THE LICENSE FEE
- third_party_indemnity @L30 :: stellt Melissa von allen Ansprüchen Dritter  [stattdessen: liability]
- penalty @L34 :: kann die Beklagte Ihnen die dadurch entstehenden Kosten pauschal berec
- unilateral_change @L38 :: Die Zustimmung des Kunden gilt als erteilt
- price_escalation @L42 :: den Preis für den D. Service an sich verändernde Marktbedingungen
- price_escalation @L50 :: durch einseitige Leistungsbestimmung gem. § 315 BGB nach billigem Erme
- price_escalation @L58 :: Von einer Beitragsanpassung kann abgesehen werden
