# Evasion-Slice: Muster-Vorschlaege (patterns_evasion.json)

Bezug: Lead-Auftrag (misses_evasion.txt auswerten, patterns_evasion.json liefern).
Engine-Revision dieses Berichts: src/analyze.js sha256 ba78dc6dc78e8d74, mtime 2026-10-15T18:37:32.781Z.
Die Datei wird parallel von anderen Slice-Besitzern geaendert; Zeilennummern driften, die Musterquellen unten sind stabil.

## Ergebnis: patterns = []

Am aktuellen Stand kann keine RegExp einen der beiden verbleibenden Fehlschlaege des Slice schliessen.
Beide betreffen Collect-Regeln, und Collect-Regeln werden von der Pass-2-Aufloesung ausgenommen.

Messung mit dem eigenen Checker (kein Schreibzugriff auf src/):

    node "Verträge/proposals/check_evasion.mjs"
    evasion: 286/288 Treffer, 2 Fehlschlaege, 0 Falschtreffer, 0 nicht zugeordnet
    MISS evasion-011:L41 notice_period (kein Befund)
    MISS evasion-023:L19 deemed_acceptance (kein Befund)

Von den 19 Fehlschlaegen des Dossiers sind 17 am aktuellen Stand geschlossen: die Anlagen-Aufloesung
attributiert die 17 Nicht-Collect-Detektoren (as_is_software_disclaimer, entire_agreement,
marketing_reference, feedback_ip_grab, fictitious_receipt, unverbindliche_sla_falle,
agency_commission_exclusion, agency_goodwill_waiver, international_data_processing,
statute_of_limitations, written_form_trap, burden_of_proof, dynamic_url_referral, vicarious_agents,
unreasonable_audit, data_lockin, third_party_indemnity) korrekt an den verweisenden Absatz.

## Fehlschlag 1: evasion-011:L41 notice_period (Split-Technik)

    L41: "(5) Der Vertrag kann von beiden Parteien mit einer Frist von,"
    L43: "(6) achtzehn Monaten zum Jahresende gekuendigt werden."

Ursache: notice_period ist eine Collect-Regel. Ihre collect()-Implementierung liest ausschliesslich
flattenSentence(s) (Zeile "const text = flattenSentence(s);" im notice_period-Block) und ignoriert
damit s.continuation. Collect-Regeln konsultieren rule.patterns nie (notice_period hat gar kein
patterns-Array), und Pass 2 ueberspringt Collect-Regeln. Ein Muster-Zusatz kann diesen Fehlschlag
deshalb grundsaetzlich nicht schliessen.

Beleg: Joinet man die beiden Absaetze (L41 + " " + L43), meldet die Engine notice_period an Zeile 41.
Die Ankerlogik ist also korrekt, nur der Satztext ist abgeschnitten.

Fix (src/, eine Stelle): in notice_period.collect den Prueftext wie collectPerSentence bilden, also
flattenSentence(s) plus s.continuation.

## Fehlschlag 2: evasion-023:L19 deemed_acceptance (Anlagen-Verweis)

Der Verweissatz "Die in Anlage 2 Ziffer 1 geregelte Massnahme gilt fuer alle Ansprueche aus diesem
Vertrag." traegt keinen eigenen Tatbestand. Die Engine loest den Verweis zwar auf (resolveAnnexItem
liefert den Anlagentext "(1) Die Abrechnung gilt als genehmigt, wenn der Kunde nicht binnen sieben
Kalendertagen nach Erhalt schriftlich widerspricht."), attributiert ihn aber nicht an den Verweissatz.

Ursache: Pass 2 ueberspringt Collect-Regeln (Zeile "if (rule.collect) continue;" in der Pass-2-Schleife
von analyzeContract). deemed_acceptance ist eine Collect-Regel.

Beleg: Entfernt man RULES.deemed_acceptance.collect zur Laufzeit, erscheint der Befund sofort am
Verweisabsatz (Mini-Text: Befunde 1-1 und 3-3 statt nur 3-3). Das bestehende Muster 5 der Regel
(gilt ... als genehmigt) matcht die Verkettung Verweissatz + Anlagentext bereits; nach dem Fix ist kein
neues Muster noetig.

Ein Muster als Ersatz funktioniert nicht: Haengt man deemed_acceptance das Muster
/Ziffer\s*\d[^.!?;:\n]{0,120}?gilt\s+f(?:ue|ü)r\s+alle\s+Anspr(?:ue|ü)che/i an, feuert es auf
allen neun Verweissatzzeilen von evasion-010 und evasion-023, also 17 Fehltreffer auf Zeilen, die
anderen Detektortypen zugeordnet sind. Verworfen.

## Verallgemeinerung: alle 9 Collect-Regeln sind blind fuer aufgeloeste Anlagenverweise

Probe (Verweissatz + Anlage mit dem jeweiligen Bibliotheks-Trap), pointer = Befund am Verweissatz:

    notice_period                         collect  alone=true  pointer=false anywhere=true
    penalty                               collect  alone=true  pointer=false anywhere=true
    non_compete                           collect  alone=true  pointer=false anywhere=true
    unilateral_change                     collect  alone=true  pointer=false anywhere=true
    deemed_acceptance                     collect  alone=true  pointer=false anywhere=true
    unilateral_convenience_termination    collect  alone=true  pointer=false anywhere=true
    price_escalation                      collect  alone=true  pointer=false anywhere=true
    asymmetric_confidentiality            collect  alone=true  pointer=false anywhere=true
    mandatory_expert_determination        collect  alone=true  pointer=false anywhere=true
    liability                             patterns alone=true  pointer=true  anywhere=true
    data_lockin                           patterns alone=true  pointer=true  anywhere=true
    written_form_trap                     patterns alone=true  pointer=true  anywhere=true

Der Fix aus Fehlschlag 2 schliesst also die ganze Klasse (9 Detektortypen x Anlagenverweis).
Hinweis zur Split-Technik: bis auf notice_period delegieren alle Collect-Regeln an
collectPerSentence(this, sentences) und sehen s.continuation; nur notice_period ist zusaetzlich
continuation-blind.

## Register-Metrik des Slice

288 Traps, 100 Prozent teilen hoechstens drei aufeinanderfolgende Woerter mit dem jeweils gematchten
Regelmuster (Median 0, Maximum 1). Der Slice ist damit weiterhin ein harter Paraphrase-Test.

## Falschtreffer-Regression (behoben)

Bei der Zwischenrevision sha256 5c30483f meldete der Checker 8 Falschtreffer auf harmlosen Klauseln
(je einer in evasion-004, -008, -012, -016, -020, -024, -028, -032) auf der Klausel
"(1) Fuer die Vertragsdauer gelten die vereinbarten Konditionen unveraendert."
Ausloeser war ein gemergtes Supplement unter excessive_survival:

    new RegExp("(?:besteh\\w+|gilt|gelten|wirkt|wirken|wird|werden|ist|sind)[^.!?;:\\n]{0,60}?\\b(?:auf\\s+dauer|unbegrenzt|unbefristet|unver(?:ä|ae)ndert|dauerhaft|zeitlich\\s+unbegrenzt)\\b", "i")

Das Muster traf 0 der 6 excessive_survival-Traps des Slice und 1 harmlose Standardklausel.
An der aktuellen Revision ba78dc6dc78e8d74 ist der Falschtreffer behoben (Checkerlauf: 0 Falschtreffer); der
Punkt bleibt hier als Regressionsnotiz.

## Verifikation

1. Muster-Vorschlaege: keine (leere Liste), Match-Test entfaellt.
2. Falschtreffer auf harmlosen Markern des Slice: 0 (Checkerlauf, Exit 0).
3. Trefferquote vor und nach dem leeren Vorschlag: 286/288, unveraendert. In-Memory-Installation ueber
   node "Verträge/proposals/check_evasion.mjs" --apply (haengt 0 Muster an, kein src-Schreibzugriff).
