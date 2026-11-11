#!/usr/bin/env node
"use strict";

import { readFileSync, writeFileSync, mkdirSync, existsSync } from "node:fs";
import { resolve, join, dirname } from "node:path";

const ROOT_DIR = resolve(".");
const TEMPLATE_PATH = join(ROOT_DIR, "klauseln", "automatische-vertragsverlaengerung.html");
const templateHtml = readFileSync(TEMPLATE_PATH, "utf8");

// Extract CSS style block
const styleMatch = templateHtml.match(/<style>([\s\S]*?)<\/style>/);
const baseCss = styleMatch ? styleMatch[1] : "";

const PAGES = [
  // --- VERTRÄGE ---
  {
    relPath: "vertraege/it-dienstleistungsvertrag-pruefen.html",
    title: "IT-Dienstleistungsvertrag prüfen (AGB-Kontrolle & Haftungsfallen)",
    metaDesc: "IT-Dienstleistungsverträge rechtssicher auf BGB-Fallen, unzulässige Haftungsklauseln, SLA-Strafen und Urheberrechte prüfen.",
    h1: "IT-Dienstleistungsvertrag: BGB-Prüfung & typische AGB-Fallen",
    category: "Vertragstypen",
    intro: "IT-Dienstleistungsverträge gehören zu den streitanfälligsten Vertragswerken im B2B-Geschäft. Oft vermischen Auftraggeber werkvertragliche Erfolgshaftung mit dienstvertraglichen Aufwandsmodellen oder bürden dem IT-Dienstleister unbegrenzte Folgeschadenhaftung auf.",
    riskHeadline: "Haftungs- & Abnahmefallen im IT-Projekt",
    riskPoints: [
      "Versteckter Werkvertrag trotz Kennzeichnung als Dienstvertrag (Gefahr unbezahlter Nacherfüllung).",
      "Vollständiger Ausschluss der Haftungsbegrenzung bei leichten Pflichtverletzungen.",
      "Unverhältnismäßig kurze Rügefristen oder fingierte Abnahmefristen zugunsten des Kunden."
    ],
    bgbRule: "§ 307 Abs. 1 & 2 BGB (Unangemessene Benachteiligung), § 611 BGB vs. § 631 BGB.",
    safeDraft: "„Die Haftung des Auftragnehmers für Schäden aus einfacher Fahrlässigkeit wird der Höhe nach auf die vertragstypisch vorhersehbaren Schäden, maximal jedoch auf die im jeweiligen Kalenderjahr vom Auftraggeber gezahlte Vergütung begrenzt. Dies gilt nicht für Schäden aus der Verletzung von Leben, Körper oder Gesundheit sowie bei vorsätzlichem oder grob fahrlässigem Handeln.“"
  },
  {
    relPath: "vertraege/freelancer-vertrag-agb-falle.html",
    title: "Freelancer-Vertrag prüfen (Scheinselbstständigkeit & Wettbewerbsverbote)",
    metaDesc: "B2B Freelancer-Verträge auf nachvertragliche Wettbewerbsverbote, Scheinselbstständigkeit-Indikatoren und unfaire Haftung prüfen.",
    h1: "Freelancer-Verträge: Risikoprüfung & Klausel-Audit",
    category: "Vertragstypen",
    intro: "Freie Entwickler und Berater unterschreiben Rahmenverträge von Agenturen oder Vermittlern oft ohne vertiefte Prüfung. Dabei finden sich regelmäßig Klauseln, die existenzbedrohende Wettbewerbsverbote oder Kundenschutzklauseln ohne Karenzentschädigung enthalten.",
    riskHeadline: "Typische AGB-Fallen in Freelancer-Verträgen",
    riskPoints: [
      "Nachvertragliche Wettbewerbsverbote ohne Entschädigungszahlung (§ 74 HGB analog).",
      "Pauschale Vertragsstrafen ohne Schadensnachweis und ohne Deckelung.",
      "Klauseln zur Weisungsgebundenheit, die das Risiko einer Scheinselbstständigkeit massiv erhöhen."
    ],
    bgbRule: "§ 307 BGB, § 74 HGB (Karenzentschädigung), § 310 BGB.",
    safeDraft: "„Der Auftragnehmer verpflichtet sich, für die Dauer von 12 Monaten nach Beendigung dieses Auftrags nicht direkt für den Endkunden in demselben Projekt tätig zu werden. Für jeden schuldhaften Verstoß wird eine angemessene, vom Auftraggeber festzusetzende und im Streitfall vom zuständigen Gericht zu überprüfende Vertragsstrafe fällig.“"
  },
  {
    relPath: "vertraege/software-saas-agb-pruefen.html",
    title: "SaaS-AGB prüfen (Verfügbarkeit, Datensouveränität & Kündigungsrechte)",
    metaDesc: "Software-as-a-Service AGB auf BGB-Konformität prüfen: Verfügbarkeitsgarantien, Lock-in-Klauseln und automatische Verlängerungen.",
    h1: "SaaS-AGB: Risikoprüfung für Cloud-Software & B2B-Plattformen",
    category: "Vertragstypen",
    intro: "Bei Cloud-Software (SaaS) verlagern Anbieter Risiken häufig einseitig auf Kunden – durch unverbindliche Verfügbarkeitsversprechen, Wartungsfenster zu Kernarbeitszeiten oder intransparente Preiseskalationsklauseln.",
    riskHeadline: "Häufige Klauselfallen bei SaaS-Verträgen",
    riskPoints: [
      "Einseitige Preisänderungsklauseln ohne Sonderkündigungsrecht nach § 308 Nr. 4 BGB.",
      "Haftungsausschluss für Datenverlust trotz fehlender kundenseitiger Backup-Möglichkeit.",
      "Ausschluss von Service-Credits bei wiederholten Verfügbarkeitsausfällen."
    ],
    bgbRule: "§ 307 BGB, § 308 Nr. 4 BGB (Änderungsvorbehalte), DSGVO Art. 28.",
    safeDraft: "„Der Anbieter gewährleistet eine durchschnittliche Systemverfügbarkeit von 99,5 % im Jahresmittel (ausschließlich angekündigter Wartungsfenster). Bei Unterschreitung der Verfügbarkeit erhält der Kunde eine anteilige Gutschrift auf die nächste Abrechnungsperiode.“"
  },
  {
    relPath: "vertraege/rahmenvertrag-haftung-it.html",
    title: "IT-Rahmenverträge prüfen (Haftungshöchstgrenzen & Freistellungen)",
    metaDesc: "B2B-Rahmenverträge im IT-Bereich auditieren: Haftungsgrenzen, Freistellungsklauseln Dritter und Kardinalpflichten nach BGB.",
    h1: "IT-Rahmenvertrag: Haftungsgrenzen & Freistellungs-Audit",
    category: "Vertragstypen",
    intro: "Rahmenverträge regeln die langfristige Zusammenarbeit. Wenn hier unklare Haftungsgrenzen oder einseitige Freistellungspflichten verankert sind, gelten diese automatisch für jeden Einzelauftrag.",
    riskHeadline: "Kritische Punkte in Rahmenverträgen",
    riskPoints: [
      "Freistellung von Ansprüchen Dritter (Indemnity) ohne Verschuldenserfordernis.",
      "Ausschluss jeglicher Haftungsdeckelung für Datenschutz- und Vertraulichkeitsverstöße.",
      "Fehlende Bestimmung zum Vorrang von Einzelvereinbarungen gegenüber dem Rahmenvertrag."
    ],
    bgbRule: "§ 307 BGB (Verstoß gegen wesentliche Grundgedanken der gesetzlichen Regelung).",
    safeDraft: "„Soweit im Einzelauftrag nicht abweichend geregelt, ist die Gesamthaftung der Parteien aus und im Zusammenhang mit dem Rahmenvertrag auf die vertragstypischen, vorhersehbaren Schäden begrenzt.“"
  },
  {
    relPath: "vertraege/nda-geheimhaltungsvereinbarung-pruefen.html",
    title: "NDA prüfen (Geheimhaltungsvereinbarung: Laufzeiten & Vertragsstrafen)",
    metaDesc: "Non-Disclosure Agreements (NDA) auf überlange Laufzeiten, unbestimmte Geschäftsgeheimnisse und Vertragsstrafenfallen prüfen.",
    h1: "Geheimhaltungsvereinbarung (NDA): Prüfung & BGB-Audit",
    category: "Vertragstypen",
    intro: "NDAs wirken oft wie Standard-Formalitäten. Einseitige NDAs mit 'ewiger' Geheimhaltungspflicht oder drakonischen Vertragsstrafen bei formellen Verstößen stellen jedoch ein massives Haftungsrisiko dar.",
    riskHeadline: "Versteckte Risiken in NDAs",
    riskPoints: [
      "Ewige Geheimhaltungspflicht ohne sachlichen Grund (übermäßige Knebelung nach § 138 BGB).",
      "Vertragsstrafen ohne Verschulden oder mit starren, unverhältnismäßig hohen Summen.",
      "Fehlende Ausnahme für gesetzliche oder behördliche Offenlegungspflichten."
    ],
    bgbRule: "GeschGehG § 1 ff., § 307 BGB, § 138 BGB (Sittenwidrigkeit).",
    safeDraft: "„Die Verpflichtung zur vertraulichen Behandlung erlischt 3 Jahre nach Beendigung der Zusammenarbeit. Informationen, die der empfangenden Partei bereits vor der Mitteilung nachweislich bekannt waren oder öffentlich zugänglich sind, unterfallen nicht der Geheimhaltungspflicht.“"
  },
  {
    relPath: "vertraege/evb-it-dienstvertrag-agb-pruefung.html",
    title: "EVB-IT Dienstvertrag prüfen (Öffentliche Auftraggeber & AGB-Besonderheiten)",
    metaDesc: "EVB-IT Verträge und ergänzende AGB öffentlicher Auftraggeber auf unzulässige Abweichungen und Risiken auditieren.",
    h1: "EVB-IT Dienstvertrag: Rechtssichere Prüfung für Auftragnehmer",
    category: "Vertragstypen",
    intro: "Bei öffentlichen Ausschreibungen sind EVB-IT Verträge Standard. Problematisch sind oft die 'Zusätzlichen Vertragsbedingungen' (ZVB) oder 'Besonderen Vertragsbedingungen' (BVB) der Vergabestellen.",
    riskHeadline: "Häufige Abweichungsrisiken bei EVB-IT",
    riskPoints: [
      "Verschärfte Gewährleistungsfristen über die Standard-EVB-IT Vorgaben hinaus.",
      "Unverhältnismäßige Vertragsstrafenregelungen für Meilensteinverzögerungen.",
      "Einseitige Nutzungsrechteübertragungen für vorbestehendes geistiges Eigentum (Background IP)."
    ],
    bgbRule: "EVB-IT Standards, § 307 BGB, VgV § 1 ff.",
    safeDraft: "„Nutzungsrechte an vorbestehender Software oder Bibliotheken des Auftragnehmers (Background IP) verbleiben beim Auftragnehmer; der Auftraggeber erhält hieran ein einfaches, zeitlich unbeschränktes Nutzungsrecht im Rahmen des Vertragszwecks.“"
  },
  {
    relPath: "vertraege/werkvertrag-softwareentwicklung-agb.html",
    title: "Software-Werkvertrag prüfen (Abnahme, Nacherfüllung & AGB)",
    metaDesc: "Werkverträge in der Individualsoftwareentwicklung auf Abnahmefiktionen, Spezifikationslücken und Mängelhaftung prüfen.",
    h1: "Software-Werkvertrag: Abnahme & Mängelhaftung im AGB-Check",
    category: "Vertragstypen",
    intro: "Im Software-Werkvertrag schuldet der Entwickler das Gelingen. Unklare Leistungsbeschreibungen in Kombination mit starren Abnahmefristen führen regelmäßig zu Honorarausfällen.",
    riskHeadline: "Kritische Vertragspunkte im Software-Werkvertrag",
    riskPoints: [
      "Ausschluss der Abnahmefiktion nach § 640 Abs. 2 BGB zu Lasten des Entwicklers.",
      "Verpflichtung zur Fehlerbeseitigung ohne klare Definition von Mängelklassen (Schweregraden).",
      "Vollständige Übertragung aller Quellcode-Rechte vor vollständiger Vergütungszahlung."
    ],
    bgbRule: "§ 640 BGB (Abnahme), § 634 BGB (Rechte bei Mängeln), § 307 BGB.",
    safeDraft: "„Das Werk gilt als abgenommen, wenn der Auftraggeber nach Bereitstellung der abnahmereifen Version nicht innerhalb von 14 Werktagen wesentliche Mängel schriftlich rügt.“"
  },
  {
    relPath: "vertraege/service-level-agreement-sla-pruefen.html",
    title: "SLA prüfen (Service Level Agreement: Pönalen & Messmethoden)",
    metaDesc: "Service Level Agreements (SLA) auf vertragswidrige Pönalen, unrealistische Reaktionszeiten und Messmethoden auditieren.",
    h1: "Service Level Agreement (SLA): Vertragsstrafen & Verfügbarkeit",
    category: "Vertragstypen",
    intro: "Ein SLA legt Reaktionszeiten und Verfügbarkeiten fest. Werden Strafen unbegrenzt kumuliert, wird der Betrieb schnell unwirtschaftlich.",
    riskHeadline: "SLA-Fallen in IT-Verträgen",
    riskPoints: [
      "Strafzahlungen ohne Obergrenze (Cap), die die monatliche Servicegebühr übersteigen.",
      "Fehlende Berücksichtigung von Fremdverschulden (z. B. Ausfall von Drittanbieter-Rechenzentren).",
      "Starre Messungen ohne Ausschluss geplanter Wartungsfenster."
    ],
    bgbRule: "§ 307 BGB, § 339 ff. BGB (Vertragsstrafe).",
    safeDraft: "„Die Gesamtsumme der Service-Gutschriften für verfehlte Service-Levels ist auf maximal 15 % der jeweiligen monatlichen Service-Gebühr begrenzt.“"
  },
  {
    relPath: "vertraege/agenturvertrag-marketing-agb-kontrolle.html",
    title: "Agenturvertrag prüfen (Kreativleistungen, Nutzungsrechte & Freigaben)",
    metaDesc: "Agenturverträge für Design, Performance-Marketing und Kampagnen auf AGB-Fallen und Nutzungsrechtsverluste prüfen.",
    h1: "Agenturvertrag: Nutzungsrechte & Freigaberisiken im Audit",
    category: "Vertragstypen",
    intro: "Marketing- und Digitalagenturen erbringen gemischte Leistungen. Ohne klare Regelung verbleiben Haftungsrisiken für Werbeaussagen oder Markenrechtsverletzungen bei der Agentur.",
    riskHeadline: "Häufige Fallstricke im Agenturvertrag",
    riskPoints: [
      "Haftung der Agentur für die wettbewerbsrechtliche Zulässigkeit der Werbemaßnahme.",
      "Ausschluss von Vergütungsansprüchen bei vorzeitiger Kündigung des Auftraggebers.",
      "Übertragung uneingeschränkter Nutzungsrechte an Entwürfen vor Zahlungseingang."
    ],
    bgbRule: "§ 649 BGB a.F. / § 648 BGB (Kündigung durch Besteller), UrhG § 31 ff.",
    safeDraft: "„Die rechtliche Prüfung von Werbeaussagen auf Vereinbarkeit mit dem Wettbewerbs- und Markenrecht obliegt dem Auftraggeber.“"
  },
  {
    relPath: "vertraege/auftragsverarbeitungsvertrag-avv-dsgvo.html",
    title: "AVV nach DSGVO prüfen (Art. 28 DSGVO: Weisungsrechte & Haftung)",
    metaDesc: "Auftragsverarbeitungsverträge (AVV) auf unzumutbare Auditkosten, unbegrenzte Freistellungen und DSGVO-Konformität prüfen.",
    h1: "Auftragsverarbeitungsvertrag (AVV): Fallstricke nach Art. 28 DSGVO",
    category: "Vertragstypen",
    intro: "Auftragsverarbeitungsverträge sind Pflicht. Oft nutzen Kunden AVVs jedoch, um übermäßige Kontrollrechte und Kostenabwälzungen durchzusetzen.",
    riskHeadline: "Überzogene Klauseln in AV-Verträgen",
    riskPoints: [
      "Vor-Ort-Audits auf Kosten des Auftragsverarbeiters ohne vorherigen Nachweis berechtigter Zweifel.",
      "Pauschale Freistellung des Auftraggebers von allen DSGVO-Bußgeldern.",
      "Unrealistische Meldefristen für Sicherheitsvorfälle (< 12 Stunden statt 72 Stunden)."
    ],
    bgbRule: "Art. 28 DSGVO, Art. 82 DSGVO, § 307 BGB.",
    safeDraft: "„Vor-Ort-Inspektionen finden nur während der üblichen Geschäftszeiten und nach einer Vorankündigung von mindestens 14 Tagen auf Kosten des Auftraggebers statt.“"
  },
  {
    relPath: "vertraege/subunternehmervertrag-it-pruefung.html",
    title: "Subunternehmervertrag IT prüfen (Back-to-Back Risiken & Haftung)",
    metaDesc: "IT-Subunternehmerverträge auf unfaire Weitergabe von Hauptvertragsrisiken (Back-to-Back) und Zahlungsverzögerungen prüfen.",
    h1: "Subunternehmervertrag IT: Back-to-Back Risiken vermeiden",
    category: "Vertragstypen",
    intro: "Hauptauftragnehmer versuchen oft, Risiken aus Kundenverträgen ungesehen auf Subunternehmer abzuwälzen ('Pay when paid').",
    riskHeadline: "Gefahren im Subunternehmervertrag",
    riskPoints: [
      "'Pay-when-paid'-Klauseln: Vergütung des Subunternehmers erst nach Zahlung des Endkunden (unwirksam nach § 307 BGB).",
      "Klauseln zur unentgeltlichen Mehrarbeit bei Verzögerungen anderer Gewerke.",
      "Einseitiges Recht des Hauptauftragnehmers zur Vertragsbeendigung ohne triftigen Grund."
    ],
    bgbRule: "§ 307 BGB, § 641 BGB (Fälligkeit der Vergütung).",
    safeDraft: "„Die Fälligkeit der Vergütung des Subunternehmers ist unabhängig davon, ob und wann der Endkunde Zahlungen an den Hauptauftragnehmer leistet.“"
  },
  {
    relPath: "vertraege/software-wartungsvertrag-pflegevertrag.html",
    title: "Software-Wartungsvertrag prüfen (Updates, Support & Preisanpassung)",
    metaDesc: "Wartungs- und Pflegeverträge für Software prüfen: Update-Pflichten, Support-Zeiten und Preisanpassungsklauseln im BGB-Check.",
    h1: "Software-Wartungsvertrag: Pflege, Updates & AGB-Regeln",
    category: "Vertragstypen",
    intro: "Wartungsverträge sichern wiederkehrende Einnahmen. Doch wenn Pflichten zur Anpassung an Drittsoftware nicht begrenzt sind, droht ein Kostengrab.",
    riskHeadline: "Typische AGB-Fallen bei Wartungsverträgen",
    riskPoints: [
      "Verpflichtung zur Anpassung an künftige Betriebssystem-Updates ohne gesonderte Vergütung.",
      "Fehlende Abgrenzung zwischen Pflege (Mängelbeseitigung) und Weiterentwicklung (Feature-Requests).",
      "Unklare Regelungen zur Beendigung von Altversionen (End of Life)."
    ],
    bgbRule: "§ 307 BGB, § 327f BGB (Aktualisierungspflicht).",
    safeDraft: "„Die Softwarepflege umfasst die Beseitigung von reproduzierbaren Fehlern der jeweils aktuellen Hauptversion. Funktionserweiterungen oder Anpassungen an geänderte Drittanbieter-Schnittstellen sind gesondert zu vereinbaren.“"
  },
  {
    relPath: "vertraege/white-label-reseller-vertrag-pruefen.html",
    title: "White-Label & Reseller-Vertrag prüfen (Vertriebsrechte & Produkthaftung)",
    metaDesc: "Reseller- und White-Label-Verträge auf Gebietsschutz, Produkthaftungsabwälzung und Kündigungsausgleichsansprüche prüfen.",
    h1: "White-Label & Reseller-Verträge: AGB & Vertriebsrisiken",
    category: "Vertragstypen",
    intro: "Wer fremde Software oder Produkte unter eigenem Namen vertreibt, wird rechtlich als Hersteller eingestuft – mit voller Haftung.",
    riskHeadline: "Kritische Punkte im White-Label Vertrag",
    riskPoints: [
      "Herstellerhaftung trifft White-Label-Partner ohne Rückgriffsmöglichkeit auf den Entwickler.",
      "Einseitige Mindestabnahmeverpflichtungen ohne Recht auf Vertragsanpassung.",
      "Fehlender Gebietsschutz trotz vereinbarter Exklusivität."
    ],
    bgbRule: "ProdHaftG § 4 Abs. 1 S. 2 (Quasi-Hersteller), HGB § 89b analog.",
    safeDraft: "„Der Softwarehersteller stellt den White-Label-Partner von allen Ansprüchen Dritter frei, die auf Sachmängeln oder Schutzrechtsverletzungen der Originalsoftware beruhen.“"
  },
  {
    relPath: "vertraege/cloud-hosting-vertrag-haftungsfallen.html",
    title: "Cloud-Hosting-Vertrag prüfen (Downtime, Rechenzentrum & Haftung)",
    metaDesc: "Hosting- und Infrastrukturverträge prüfen: Datenverlust, Notfall-Wiederherstellung (Disaster Recovery) und Haftungshöchstgrenzen.",
    h1: "Cloud-Hosting-Vertrag: Haftungsfallen bei Ausfall & Datenverlust",
    category: "Vertragstypen",
    intro: "Wenn Server ausfallen oder Daten verloren gehen, verweisen Hoster gerne auf ihre AGB. Doch viele Haftungsausschlüsse sind vor deutschen Gerichten unwirksam.",
    riskHeadline: "Häufige Fallstricke bei Hosting-AGB",
    riskPoints: [
      "Vollständiger Ausschluss der Haftung für Datenverlust bei fahrlässiger Löschung.",
      "Verweigerung der Herausgabe von Daten-Dumps nach Beendigung des Vertrags.",
      "Einseitige Änderungsrechte für Server-Standorte außerhalb der EU."
    ],
    bgbRule: "§ 307 BGB, § 309 Nr. 7 BGB, DSGVO Art. 44 ff.",
    safeDraft: "„Der Hoster haftet für Datenverlust bis zur Höhe des Schadens, der auch bei ordnungsgemäßer, täglicher Datensicherung durch den Kunden entstanden wäre.“"
  },
  {
    relPath: "vertraege/beratungsvertrag-consulting-agb.html",
    title: "Beratungsvertrag prüfen (Consulting: Erfolgshaftung & Verschwiegenheit)",
    metaDesc: "Consulting- und Beratungsverträge auf unerwünschte Erfolgshaftung, Mehraufwandsklauseln und Konventionalstrafen prüfen.",
    h1: "Beratungsvertrag: Dienstleistung vs. Erfolgshaftung im AGB-Check",
    category: "Vertragstypen",
    intro: "Unternehmensberater schulden die fachgerechte Beratung, nicht das wirtschaftliche Ergebnis. Häufig versuchen Auftraggeber, Berater auf Kennzahlen festzunageln.",
    riskHeadline: "Risiken im Consulting-Vertrag",
    riskPoints: [
      "Vergütungsrückzahlung bei Nichterreichen bestimmter betriebswirtschaftlicher Kennzahlen.",
      "Übermäßige Dokumentationspflichten als Voraussetzung für Fälligkeit des Honorars.",
      "Unklar formulierte Pflichten zur Anwesenheit vor Ort beim Kunden."
    ],
    bgbRule: "§ 611 BGB, § 307 BGB.",
    safeDraft: "„Gegenstand des Vertrags ist die Erbringung der vereinbarten Beratungsleistung, nicht die Erzielung eines bestimmten wirtschaftlichen Erfolgs.“"
  },
  {
    relPath: "vertraege/api-lizenzvertrag-nutzungsbedingungen.html",
    title: "API-Lizenzvertrag prüfen (Rate Limits, Deprecation & IP-Rechte)",
    metaDesc: "API-Nutzungsbedingungen und Lizenzverträge auf einseitige Abschaltungsrechte, Rate-Limit-Strafen und Datenschutz auditieren.",
    h1: "API-Lizenzvertrag: Rate Limits & Nutzungsrechte im Audit",
    category: "Vertragstypen",
    intro: "Wer seine Geschäftsmodelle auf Dritt-APIs aufbaut, riskiert bei unfairen AGB den plötzlichen Verlust des Systemzugangs.",
    riskHeadline: "Typische AGB-Fallen in API-Bedingungen",
    riskPoints: [
      "Recht zur sofortigen, unangekündigten Abschaltung der Schnittstelle ohne Übergangsfrist.",
      "Anspruch des API-Anbieters auf Nutzungsrechte an Daten, die über die API übertragen werden.",
      "Verbot des Cachings von API-Antworten trotz zwingender technischer Notwendigkeit."
    ],
    bgbRule: "§ 307 BGB, § 308 Nr. 4 BGB.",
    safeDraft: "„Änderungen der API-Spezifikation oder die Einstellung von Schnittstellen werden mit einer Frist von mindestens 90 Tagen im Voraus angekündigt.“"
  },
  {
    relPath: "vertraege/open-source-software-compliance-vertrag.html",
    title: "Open-Source-Compliance prüfen (GPL, Copyleft & B2B-Lieferverträge)",
    metaDesc: "B2B-Lieferverträge auf Open-Source-Klauseln prüfen: Copyleft-Risiken, Pflicht zur Quellcode-Herausgabe und Gewährleistung.",
    h1: "Open-Source-Compliance: Copyleft-Risiken in Lieferverträgen",
    category: "Vertragstypen",
    intro: "Die Verwendung von Open-Source-Komponenten ist Standard. In B2B-Verträgen verlangen Kunden oft Garantien, die mit OSS-Lizenzen kollidieren.",
    riskHeadline: "Gefahrenpunkte bei Open-Source-Regelungen",
    riskPoints: [
      "Pauschale Garantie des Entwicklers, dass keine Open-Source-Software enthalten ist (oft unrealistisch).",
      "Verschleppte Lizenzverletzungen, die zu Rückrufen oder Schadensersatzforderungen führen.",
      "Fehlende Regelungen zur Trennung von proprietärem Code und Copyleft-Bibliotheken (GPLv3)."
    ],
    bgbRule: "UrhG § 69a ff., § 307 BGB.",
    safeDraft: "„Der Auftragnehmer sichert zu, dass die gelieferte Software den Lizenzbedingungen der verwendeten Open-Source-Komponenten entspricht und keine Copyleft-Verpflichtungen für den proprietären Code des Auftraggebers auslöst.“"
  },
  {
    relPath: "vertraege/einkaufsbedingungen-b2b-abwehrklausel.html",
    title: "Einkaufsbedingungen B2B prüfen (Abwehrklauseln & kollidierende AGB)",
    metaDesc: "Allgemeine Einkaufsbedingungen (AEB) prüfen: Battle of Forms, kollidierende Klauseln und Abwehrklauseln nach BGB.",
    h1: "Einkaufsbedingungen (AEB) im B2B: Kollidierende AGB prüfen",
    category: "Vertragstypen",
    intro: "Wenn Verkaufs-AGB auf Einkaufs-AGB treffen ('Battle of the Forms'), greifen oft widersprüchliche Abwehrklauseln.",
    riskHeadline: "Häufige Fallstricke in AEB",
    riskPoints: [
      "Stillschweigende Unterwerfung unter die AEB des Kunden durch vorbehaltlose Lieferung.",
      "Einseitige Verlängerung von Gewährleistungsfristen auf 3 oder 5 Jahre.",
      "Ausschluss des Eigentumsvorbehalts des Lieferanten."
    ],
    bgbRule: "§ 305 Abs. 2 BGB, § 306 BGB, BGH-Rechtsprechung zur Kongruenzlehre.",
    safeDraft: "„Widersprechende Bedingungen des Kunden werden nicht anerkannt, es sei denn, ihrer Geltung wird ausdrücklich schriftlich zugestimmt. Die Durchführung der Lieferung gilt nicht als Zustimmung.“"
  },

  // --- BGB-GRUNDLAGEN ---
  {
    relPath: "bgb/305-einbeziehung-agb-vorvertragliche-pflichten.html",
    title: "§ 305 BGB: Einbeziehung von AGB in den Vertrag",
    metaDesc: "Wann sind AGB wirksam einbezogen? Unterschiede zwischen B2B und B2C, Hinweispflichten und zumutbare Kenntnisnahme.",
    h1: "§ 305 BGB: Voraussetzungen für die wirksame Einbeziehung von AGB",
    category: "BGB Grundlagen",
    intro: "Damit Allgemeine Geschäftsbedingungen Vertragsbestandteil werden, müssen sie wirksam einbezogen worden sein. Im kaufmännischen Verkehr (B2B) gelten hierfür erleichterte Bedingungen, aber keine Narrenfreiheit.",
    riskHeadline: "Fehlerquellen bei der AGB-Einbeziehung",
    riskPoints: [
      "Reiner Hinweis auf der Rechnung nach Vertragsschluss (zu spät!).",
      "Versteckter Link in der E-Mail-Signatur ohne deutlichen Hinweis im Angebot.",
      "Nichteinbeziehung führt dazu, dass das gesetzliche BGB gilt – oft zum Nachteil des Verwenders."
    ],
    bgbRule: "§ 305 Abs. 2 BGB, § 305 Abs. 3 BGB (Rahmenvereinbarungen).",
    safeDraft: "„Es gelten ausschließlich unsere Allgemeinen Geschäftsbedingungen in der bei Vertragsschluss gültigen Fassung. Der Kunde kann diese jederzeit unter [URL] abrufen und speichern.“"
  },
  {
    relPath: "bgb/305b-vorrang-der-individualabrede.html",
    title: "§ 305b BGB: Vorrang der Individualabrede vor AGB",
    metaDesc: "Gilt das mündliche Versprechen oder das Kleingedruckte? Warum individuelle Vereinbarungen jede AGB-Klausel schlagen.",
    h1: "§ 305b BGB: Der Vorrang der Individualabrede im Vertragsrecht",
    category: "BGB Grundlagen",
    intro: "Ein Grundsatz des deutschen Zivilrechts: Was die Vertragsparteien individuell aushandeln, hat immer Vorrang vor vorformulierten Standardklauseln – egal, was in den AGB steht.",
    riskHeadline: "Irrtümer zum Vorrang der Individualabrede",
    riskPoints: [
      "Schriftformklauseln in AGB können mündliche Individualabsprachen nicht verhindern.",
      "E-Mail-Zusagen von Vertriebsmitarbeitern setzen gedruckte AGB außer Kraft.",
      "Beweislast liegt bei der Partei, die sich auf die mündliche Vereinbarung beruft."
    ],
    bgbRule: "§ 305b BGB, BGH-Rechtsprechung zu doppelten Schriftformklauseln.",
    safeDraft: "„Individuelle Vereinbarungen der Parteien haben Vorrang vor diesen Geschäftsbedingungen. Für den Inhalt derartiger Vereinbarungen ist, vorbehaltlich des Gegenbeweises, ein schriftlicher Vertrag oder eine schriftliche Bestätigung maßgebend.“"
  },
  {
    relPath: "bgb/305c-ueberraschende-mehrdeutige-klauseln.html",
    title: "§ 305c BGB: Überraschende und mehrdeutige Klauseln",
    metaDesc: "Wann ist eine AGB-Klausel überraschend oder mehrdeutig? Die Unklarheitenregel und das Überrumpelungsverbot im BGB.",
    h1: "§ 305c BGB: Überraschende Klauseln & Unklarheitenregel",
    category: "BGB Grundlagen",
    intro: "Klauseln, die so ungewöhnlich sind, dass der Vertragspartner mit ihnen nicht zu rechnen brauchte, werden nicht Vertragsbestandteil. Zweifel bei der Auslegung gehen stets zu Lasten des Verwenders.",
    riskHeadline: "Anwendungsfälle von § 305c BGB",
    riskPoints: [
      "Verstecken von gravierenden Pflichten an unpassender Stelle im Fließtext.",
      "Mehrdeutige Formulierungen werden vom Gericht immer in der kundenfreundlichsten Variante ausgelegt.",
      "Gilt uneingeschränkt auch im kaufmännischen Geschäftsverkehr (B2B)."
    ],
    bgbRule: "§ 305c Abs. 1 BGB (Überraschende Klauseln), § 305c Abs. 2 BGB (Unklarheitenregel).",
    safeDraft: "„Wesentliche Vertragspflichten, Laufzeiten und Vergütungsmodalitäten sind klar, verständlich und an prominenter Stelle im Hauptvertrag hervorzuheben.“"
  },
  {
    relPath: "bgb/306-rechtsfolgen-bei-nichteinbeziehung-unwirksamkeit.html",
    title: "§ 306 BGB: Rechtsfolgen bei Unwirksamkeit von AGB-Klauseln",
    metaDesc: "Was passiert, wenn eine Klausel kippt? Verbot der geltungserhaltenden Reduktion und Rückfall auf das gesetzliche BGB.",
    h1: "§ 306 BGB: Rechtsfolgen bei unzulässigen Klauseln",
    category: "BGB Grundlagen",
    intro: "Ist eine AGB-Klausel unwirksam, bleibt der Vertrag im Übrigen wirksam. Die Lücke wird durch gesetzliche Vorschriften gefüllt – eine salvatorische Nachbesserung durch Gerichte findet nicht statt.",
    riskHeadline: "Gefahren für Verwender unzulässiger Klauseln",
    riskPoints: [
      "Verbot der geltungserhaltenden Reduktion: Eine zu weit gefasste Klausel ist komplett unwirksam.",
      "Gesetzliches Gewährleistungs- und Haftungsrecht greift voll durch (oft viel strenger als gewollt).",
      "Kostenrisiko durch Abmahnungen von Wettbewerbern oder Verbänden."
    ],
    bgbRule: "§ 306 Abs. 1 & 2 BGB, ständige BGH-Rechtsprechung.",
    safeDraft: "„Soweit Bestimmungen dieses Vertrags unwirksam sind oder werden, richtet sich der Inhalt des Vertrags nach den gesetzlichen Vorschriften.“"
  },
  {
    relPath: "bgb/307-inhaltskontrolle-treu-und-glauben.html",
    title: "§ 307 BGB: Inhaltskontrolle nach Treu und Glauben",
    metaDesc: "Die Generalklausel der AGB-Prüfung: Unangemessene Benachteiligung, Transparenzgebot und Anwendung im B2B-Bereich.",
    h1: "§ 307 BGB: Die Inhaltskontrolle von AGB-Klauseln",
    category: "BGB Grundlagen",
    intro: "§ 307 BGB ist das schärfste Schwert im AGB-Recht. Auch im B2B-Verkehr prüft die Rechtsprechung jede Klausel darauf, ob sie den Partner entgegen Treu und Glauben unangemessen benachteiligt.",
    riskHeadline: "Prüfungsmaßstäbe nach § 307 BGB",
    riskPoints: [
      "Abweichung von wesentlichen Grundgedanken der gesetzlichen Regelung (z. B. Haftung für Hauptpflichten).",
      "Einschränkung von Kardinalpflichten, die die Erreichung des Vertragszwecks gefährdet.",
      "Verstoß gegen das Transparenzgebot (unklare, verschachtelte Regelungen)."
    ],
    bgbRule: "§ 307 Abs. 1 BGB (Generalklausel & Transparenzgebot), § 307 Abs. 2 BGB (Regelbeispiele).",
    safeDraft: "„Bestimmungen sind so zu fassen, dass die Rechte und Pflichten der Vertragspartner ausgewogen bleiben und die Risikoverteilung dem Leitbild des jeweiligen Vertragstyps entspricht.“"
  },
  {
    relPath: "bgb/308-klauselverbote-mit-wertungsmoeglichkeit.html",
    title: "§ 308 BGB: Klauselverbote mit Wertungsmöglichkeit",
    metaDesc: "Welche Fristen, Rücktrittsvorbehalte und Änderungsvorbehalte sind unzulässig? Wertungsmaßstäbe nach § 308 BGB.",
    h1: "§ 308 BGB: Klauselverbote mit Wertungsmöglichkeit im Überblick",
    category: "BGB Grundlagen",
    intro: "§ 308 BGB enthält einen Katalog von Klauseln, die bei unzureichender sachlicher Rechtfertigung unwirksam sind, wie etwa überlange Annahmefristen oder einseitige Leistungsänderungen.",
    riskHeadline: "Kritische Klauseltypen nach § 308 BGB",
    riskPoints: [
      "Unangemessen lange oder unbestimmte Fristen für die Annahme eines Angebots (§ 308 Nr. 1 BGB).",
      "Vorbehalt, die versprochene Leistung ohne triftigen Grund zu ändern (§ 308 Nr. 4 BGB).",
      "Fingierte Erklärungen bei Schweigen des Vertragspartners (§ 308 Nr. 5 BGB)."
    ],
    bgbRule: "§ 308 BGB (Klauselkatalog 1 bis 9).",
    safeDraft: "„Leistungsänderungen sind nur zulässig, soweit sie für den Kunden zumutbar sind und auf sachlichen, bei Vertragsschluss nicht vorhersehbaren Gründen beruhen.“"
  },
  {
    relPath: "bgb/309-klauselverbote-ohne-wertungsmoeglichkeit.html",
    title: "§ 309 BGB: Klauselverbote ohne Wertungsmöglichkeit",
    metaDesc: "Haftungsausschlüsse, pauschalierte Schadensersatzansprüche und Vertragsstrafen: Die harten Verbote des § 309 BGB.",
    h1: "§ 309 BGB: Absolute Klauselverbote im deutschen Recht",
    category: "BGB Grundlagen",
    intro: "Die Verbote des § 309 BGB sind absolut ('Blacklist'). Im B2C-Verkehr führen sie ohne jeden Ermessensspielraum zur Nichtigkeit. Im B2B-Verkehr dienen sie als strenger Maßstab für § 307 BGB.",
    riskHeadline: "Die wichtigsten Verbote aus § 309 BGB",
    riskPoints: [
      "Haftungsausschluss bei Verletzung von Leben, Körper, Gesundheit oder grobem Verschulden (§ 309 Nr. 7 BGB).",
      "Ausschluss des Rechts, sich bei Pflichtverletzungen vom Vertrag zu lösen (§ 309 Nr. 8 BGB).",
      "Pauschalierung von Schadensersatzansprüchen ohne Möglichkeit des Gegenbeweises (§ 309 Nr. 5 BGB)."
    ],
    bgbRule: "§ 309 BGB (Nr. 1 bis 13).",
    safeDraft: "„Der Schadensersatzanspruch bleibt auf den tatsächlichen Schaden begrenzt. Dem Kunden steht ausdrücklich der Nachweis frei, dass ein Schaden überhaupt nicht oder wesentlich niedriger als die Pauschale entstanden ist.“"
  },
  {
    relPath: "bgb/310-anwendungsbereich-b2b-kaufleute.html",
    title: "§ 310 BGB: AGB-Recht im B2B-Bereich & für Kaufleute",
    metaDesc: "Welche AGB-Regeln gelten zwischen Unternehmen? Grenzen der Vertragsfreiheit nach § 310 Abs. 1 BGB und Handelsbräuche.",
    h1: "§ 310 BGB: AGB-Kontrolle zwischen Unternehmen (B2B)",
    category: "BGB Grundlagen",
    intro: "Unter Kaufleuten gilt Vertragsfreiheit – aber nicht schrankenlos. Über § 310 Abs. 1 BGB finden die Wertungen der §§ 308, 309 BGB über die Generalklausel § 307 BGB fast vollständige Anwendung.",
    riskHeadline: "Häufige Fehlannahmen im B2B-Vertragsrecht",
    riskPoints: [
      "Der Glaube, dass zwischen Unternehmen 'alles vereinbart werden darf'.",
      "Ausschlüsse von Gewährleistung oder Haftung werden auch im B2B regelmäßig für nichtig erklärt.",
      "Handelsbräuche (§ 310 Abs. 1 S. 2 BGB) rechtfertigen nur selten einseitige Knebelklauseln."
    ],
    bgbRule: "§ 310 Abs. 1 BGB, BGH-Grundsatzurteile zur B2B-Inhaltskontrolle.",
    safeDraft: "„Auch in B2B-Verträgen ist eine sorgfältige Abwägung erforderlich; Haftungsausschlüsse für wesentliche Vertragspflichten sind stets unwirksam.“"
  },
  {
    relPath: "bgb/haftungsbegrenzung-kardinalpflichten-muster.html",
    title: "Haftungsbegrenzung & Kardinalpflichten: Musterklausel BGB",
    metaDesc: "Wie formuliert man eine rechtssichere Haftungsbegrenzung in AGB? Musterklausel, Kardinalpflichten und BGH-Rechtsprechung.",
    h1: "Haftungsbegrenzung & Kardinalpflichten: Rechtssichere Formulierung",
    category: "BGB Grundlagen",
    intro: "Wer seine Haftung in AGB pauschal ausschließt, haftet am Ende unbegrenzt. Eine wirksame Begrenzung muss zwingend zwischen Kardinalpflichten und einfachen Pflichten unterscheiden.",
    riskHeadline: "Anforderungen des BGH an Haftungsklauseln",
    riskPoints: [
      "Haftung für Verletzung wesentlicher Vertragspflichten (Kardinalpflichten) darf nicht ausgeschlossen werden.",
      "Begrenzung ist nur auf den vertragstypisch vorhersehbaren Schaden zulässig.",
      "Vorsatz und grobe Fahrlässigkeit müssen ausdrücklich ausgenommen werden."
    ],
    bgbRule: "BGH NJW 2013, 2959; § 307 BGB, § 309 Nr. 7 BGB.",
    safeDraft: "„Für leichte Fahrlässigkeit haftet der Auftragnehmer nur bei Verletzung einer wesentlichen Vertragspflicht (Kardinalpflicht). In diesem Fall ist die Haftung der Höhe nach auf den vertragstypisch vorhersehbaren Schaden begrenzt.“"
  },
  {
    relPath: "bgb/vertragsstrafe-bgb-343-angemessenheit.html",
    title: "Vertragsstrafe im B2B (§ 343 BGB): Angemessenheit & Grenzen",
    metaDesc: "Wann ist eine Vertragsstrafe in AGB unwirksam? Obergrenzen, Verschuldenserfordernis und Herabsetzung nach § 343 BGB.",
    h1: "Vertragsstrafen in AGB: Rechtliche Grenzen & § 343 BGB",
    category: "BGB Grundlagen",
    intro: "Vertragsstrafenklauseln in AGB unterliegen einer extrem strengen Kontrolle. Fehlt eine Begrenzung nach oben oder ein Verschuldenserfordernis, ist die Klausel nach § 307 BGB komplett nichtig.",
    riskHeadline: "Voraussetzungen wirksamer Vertragsstrafen in AGB",
    riskPoints: [
      "Verschuldensunabhängige Vertragsstrafen in AGB sind ausnahmslos unwirksam.",
      "Fehlende Deckelung (z. B. auf maximal 5 % der Auftragssumme) führt zur Gesamtnichtigkeit.",
      "Kombination aus Verzugsschaden und Vertragsstrafe darf nicht zu Doppelbestrafung führen."
    ],
    bgbRule: "§ 343 BGB, § 339 BGB, § 307 BGB (BGH ständige Rechtsprechung).",
    safeDraft: "„Die Vertragsstrafe für Verzug beträgt für jeden vollen Werktag der Verzögerung 0,2 % des Netto-Auftragswerts, insgesamt jedoch höchstens 5 % des Gesamtauftragswerts.“"
  },
  {
    relPath: "bgb/verzugszinsen-bgb-288-absatz-2.html",
    title: "B2B-Verzugszinsen (§ 288 Abs. 2 BGB): 9 Prozentpunkte über Basis",
    metaDesc: "Verzugszinsen zwischen Unternehmen: 9 Prozentpunkte über dem Basiszinssatz und die 40-Euro-Verzugspauschale nach § 288 Abs. 5 BGB.",
    h1: "Verzugszinsen im B2B: Gesetzlicher Zinssatz & 40 € Pauschale",
    category: "BGB Grundlagen",
    intro: "Zahlt ein gewerblicher Kunde verspätet, schuldet er kraft Gesetzes Verzugszinsen in Höhe von 9 Prozentpunkten über dem Basiszinssatz sowie eine pauschale Entschädigung von 40 Euro.",
    riskHeadline: "Häufige Fehler bei Zinsklauseln",
    riskPoints: [
      "Vereinbarung von niedrigeren Zinsen in den AGB des Lieferanten als gesetzlich vorgesehen.",
      "Unzulässiger Ausschluss der 40-Euro-Verzugspauschale (§ 288 Abs. 5 BGB) in Kunden-AEB.",
      "Verzug tritt bei Fälligkeit auch ohne Mahnung nach 30 Tagen ab Rechnungszugang ein."
    ],
    bgbRule: "§ 288 Abs. 2 BGB (B2B-Zinssatz), § 288 Abs. 5 BGB (40 € Verzugspauschale).",
    safeDraft: "„Bei Zahlungsverzug ist der Rechnungsbetrag mit dem gesetzlichen Verzugszinssatz von 9 Prozentpunkten über dem jeweiligen Basiszinssatz zu verzinsen. Die Geltendmachung eines höheren Verzugsschadens sowie der Pauschale nach § 288 Abs. 5 BGB bleibt vorbehalten.“"
  },
  {
    relPath: "bgb/aufrechnungsverbot-b2b-rechtsprechung.html",
    title: "Aufrechnungsverbote in AGB: BGH-Rechtsprechung im B2B",
    metaDesc: "Sind Aufrechnungsverbote in B2B-Verträgen wirksam? Warum unbestrittene und rechtskräftige Forderungen ausgenommen werden müssen.",
    h1: "Aufrechnungsverbot in AGB: Rechtssichere Gestaltung nach BGB",
    category: "BGB Grundlagen",
    intro: "Unternehmen wollen verhindern, dass Kunden Zahlungen mit fragwürdigen Gegenforderungen aufrechnen. Doch ein zu weites Aufrechnungsverbot ist nach ständiger Rechtsprechung nichtig.",
    riskHeadline: "Voraussetzungen für ein wirksames Aufrechnungsverbot",
    riskPoints: [
      "Unbestrittene oder rechtskräftig festgestellte Forderungen müssen ausdrücklich von dem Verbot ausgenommen sein.",
      "Gegenforderungen aus demselben synallagmatischen Vertragsverhältnis dürfen nicht blockiert werden.",
      "Unwirksame Aufrechnungsklauseln kippen ersatzlos – der Kunde darf dann voll aufrechnen."
    ],
    bgbRule: "BGH Urteil vom 20.03.2014 – VII ZR 209/13; § 307 BGB, § 309 Nr. 3 BGB.",
    safeDraft: "„Der Kunde darf nur mit solchen Gegenansprüchen aufrechnen oder ein Zurückbehaltungsrecht geltend machen, die unbestritten, rechtskräftig festgestellt oder entscheidungsreif sind.“"
  },
  {
    relPath: "bgb/kuendigungsrecht-wichtiger-grund-bgb-314.html",
    title: "Kündigung aus wichtigem Grund (§ 314 BGB) in AGB",
    metaDesc: "Kann das Recht zur fristlosen Kündigung in AGB eingeschränkt oder erweitert werden? Voraussetzungen des § 314 BGB.",
    h1: "Fristlose Kündigung aus wichtigem Grund (§ 314 BGB) in AGB",
    category: "BGB Grundlagen",
    intro: "Das Recht, Dauerschuldverhältnisse bei Vorliegen eines wichtigen Grundes außerordentlich zu kündigen, ist zwingendes Recht und kann durch AGB weder abbedungen noch unverhältnismäßig erschwert werden.",
    riskHeadline: "Fallstricke bei Kündigungsklauseln",
    riskPoints: [
      "Ausschluss oder Einschränkung des gesetzlichen Kündigungsrechts nach § 314 BGB ist unwirksam.",
      "Einseitige Kataloge von 'wichtigen Gründen' binden Gerichte im Streitfall nicht.",
      "In der Regel ist vor einer Kündigung eine schriftliche Abmahnung mit Fristsetzung erforderlich."
    ],
    bgbRule: "§ 314 BGB (Kündigung aus wichtigem Grund), § 307 BGB.",
    safeDraft: "„Das Recht beider Parteien zur außerordentlichen Kündigung aus wichtigem Grund bleibt unberührt. Ein wichtiger Grund liegt insbesondere vor, wenn die jeweils andere Partei wesentliche Vertragspflichten trotz schriftlicher Abmahnung fortgesetzt verletzt.“"
  },

  // --- BRANCHEN-LEITFÄDEN ---
  {
    relPath: "branchen/it-systemhaeuser-haftungsrisiken.html",
    title: "AGB-Leitfaden für IT-Systemhäuser: Haftungsrisiken minimieren",
    metaDesc: "Kompakter Praxis-Leitfaden für IT-Systemhäuser: SLA-Fallen, Haftungsobergrenzen und BGB-konforme AGB im Kundenkontakt.",
    h1: "IT-Systemhäuser: Haftungsrisiken & AGB-Prüfung in der Praxis",
    category: "Branchen-Guides",
    intro: "IT-Systemhäuser tragen operative Verantwortung für Kundennetzwerke. Bei Ausfällen drohen immense Schadensersatzforderungen, wenn Rahmenverträge und AGB nicht sauber aufgestellt sind.",
    riskHeadline: "Die 3 größten Gefahrenquellen für Systemhäuser",
    riskPoints: [
      "Unklare Abgrenzung zwischen Managed Services (Dienstvertrag) und Migrationen (Werkvertrag).",
      "Übernahme unbeschränkter Haftung für Ransomware- oder Cyber-Angriffe im Kundennetzwerk.",
      "Mangelhafte Regelung von Mitwirkungspflichten des Kunden (z. B. Bereitstellung von Zugängen)."
    ],
    bgbRule: "§ 611 BGB, § 631 BGB, § 307 BGB.",
    safeDraft: "„Der Kunde ist verpflichtet, regelmäßige Datensicherungen durchzuführen und Systeme vor Übergabe an den Dienstleister zu sichern.“"
  },
  {
    relPath: "branchen/freelancer-entwickler-agb-leitfaden.html",
    title: "Vertrags-Leitfaden für Freelancer & Entwickler (B2B)",
    metaDesc: "Rechtssichere Verträge für selbstständige Entwickler: Urheberrechte, Zahlungsziele und Haftungsbeschränkung.",
    h1: "Freelancer & Software-Entwickler: AGB-Check & Schutz vor Knebelverträgen",
    category: "Branchen-Guides",
    intro: "Freelancer verhandeln oft mit finanzstarken Kunden oder Agenturen. Dieser Leitfaden zeigt, welche Klauseln Sie niemals ungeprüft unterschreiben sollten.",
    riskHeadline: "Typische Knebelklauseln gegen Freelancer",
    riskPoints: [
      "Abtretung aller gegenwärtigen und künftigen Erfindungen ohne Zusatzhonorar.",
      "Zahlungsziele von mehr als 60 Tagen ab Rechnungsstellung.",
      "Klauseln zur unentgeltlichen Mängelbeseitigung bei rein agilen Dienstverträgen."
    ],
    bgbRule: "UrhG § 31, § 271a BGB (Zahlungsfristen), § 307 BGB.",
    safeDraft: "„Rechnungen sind innerhalb von 14 Tagen nach Zugang ohne Abzug zur Zahlung fällig.“"
  },
  {
    relPath: "branchen/digitalagenturen-werkvertrag-risiken.html",
    title: "Leitfaden für Digitalagenturen: Werkvertragsrisiken & Freigaben",
    metaDesc: "AGB-Praxis für Web-, Design- und Werbeagenturen: Korrekturschleifen, Freigabefiktionen und Urheberrechte absichern.",
    h1: "Digitalagenturen: Werkvertragliche Risiken & Abnahmefallen beherrschen",
    category: "Branchen-Guides",
    intro: "Agenturen kennen das Problem endloser Korrekturschleifen ohne Vergütungsanspruch. Mit präzisen AGB und klaren Abnahmeregeln schützen Sie Ihre Liquidität.",
    riskHeadline: "Häufige Verluste durch fehlende AGB-Regeln",
    riskPoints: [
      "Fehlende Begrenzung von Korrekturschleifen im Pauschalangebot.",
      "Verweigerung der Endabnahme wegen geschmacklicher Nichtgefallen.",
      "Haftung für vom Kunden bereitgestellte Texte, Bilder oder Schriften."
    ],
    bgbRule: "§ 640 BGB (Abnahme), UrhG § 31, § 307 BGB.",
    safeDraft: "„Soweit nicht anders vereinbart, sind im Festpreisangebot maximal zwei Korrekturschleifen enthalten. Zusätzliche Anpassungen werden nach Aufwand vergütet.“"
  },
  {
    relPath: "branchen/saas-anbieter-agb-musterpruefung.html",
    title: "SaaS-Anbieter AGB: Musterprüfung & BGB-Compliance",
    metaDesc: "Rechtssichere B2B SaaS-AGB gestalten: SLAs, Cloud-Verfügbarkeit, DSGVO und Haftungsbeschränkung für Software-Startups.",
    h1: "SaaS-Anbieter: AGB-Gestaltung & Fallstricke für B2B-Software",
    category: "Branchen-Guides",
    intro: "Als SaaS-Startup brauchen Sie AGB, die international skalieren und gleichzeitig den strengen Vorgaben der deutschen Rechtsprechung standhalten.",
    riskHeadline: "Typische Abmahnfallen für SaaS-AGB",
    riskPoints: [
      "Automatisches Opt-In für Marketing-Kommunikation in den Nutzungsbedingungen.",
      "Versteckte Preiserhöhungsmechanismen ohne Sonderkündigungsrecht.",
      "Unzulässige Klauseln zum Verfall von Guthaben oder ungenutzten Kontingenten."
    ],
    bgbRule: "§ 308 Nr. 4 BGB, DSGVO Art. 6, § 307 BGB.",
    safeDraft: "„Preisanpassungen werden dem Kunden mindestens 6 Wochen vor ihrem Inkrafttreten in Textform mitgeteilt. Widerspricht der Kunde nicht innerhalb von 4 Wochen, gilt die Anpassung als genehmigt.“"
  },
  {
    relPath: "branchen/kanzleien-legaltech-vorpruefung.html",
    title: "Legal-Tech für Kanzleien: Automatisierte Vertrags-Vorprüfung",
    metaDesc: "Wie Wirtschaftskanzleien mit lokaler KI und deterministischer Klausel-Analyse B2B-Verträge in Sekunden vorprüfen.",
    h1: "Kanzleien & Rechtsabteilungen: B2B-Vertragsprüfung mit Legal-Tech",
    category: "Branchen-Guides",
    intro: "Die manuelle Durchsicht von 40-seitigen AGB-Katalogen bindet wertvolle Anwaltszeit. ClauseLens liefert eine deterministische Erstprüfung ohne Cloud-Risiken.",
    riskHeadline: "Herausforderungen für Kanzleien",
    riskPoints: [
      "Verbot der Weitergabe von Mandantendaten an US-Cloud-LLMs nach § 43e BRAO / DSGVO.",
      "Hohe Fehlerquote bei rein stichprobenartiger manueller Prüfung langer Klauselwerke.",
      "Bedarf an standardisierten Redline-Vorschlägen für Junior-Associates."
    ],
    bgbRule: "§ 43e BRAO, DSGVO Art. 28, § 305–310 BGB.",
    safeDraft: "„Durch lokale, air-gapped Prüfung bleiben alle sensiblen Mandantenunterlagen zu 100 % im Arbeitsspeicher des Prüfers.“"
  },
  {
    relPath: "branchen/einkaufsabteilungen-kmu-agb-pruefung.html",
    title: "Einkaufsabteilungen im KMU: Lieferanten-AGB schnell prüfen",
    metaDesc: "Praxis-Guide für den Mittelstandseinkauf: Wie Einkäufer Lieferanten-AGB in Sekunden auf Knebelklauseln und Preisrisiken prüfen.",
    h1: "Einkauf im Mittelstand: Lieferanten-AGB & AEB-Kollision prüfen",
    category: "Branchen-Guides",
    intro: "Einkäufer stehen täglich vor Dutzenden Auftragsbestätigungen mit angehängten AGB. Wer hier nicht aufpasst, akzeptiert einseitige Preiseskalationen und Haftungsausschlüsse.",
    riskHeadline: "Typische Einkaufsrisiken",
    riskPoints: [
      "Lieferzeitverlängerungen ohne verbindlichen Liefertermin.",
      "Einseitige Rohstoffpreis-Gleitklauseln zu Lasten des Einkäufers.",
      "Verkürzte Rügefristen für verdeckte Mängel."
    ],
    bgbRule: "§ 377 HGB (Rügeobliegenheit), § 307 BGB.",
    safeDraft: "„Versteckte Mängel können innerhalb einer Frist von mindestens 14 Tagen nach ihrer Entdeckung gerügt werden.“"
  },

  // --- INTERAKTIVE TOOLS & RECHNER ---
  {
    relPath: "tools/kuendigungsfrist-rechner-bgb.html",
    title: "Kündigungsfrist-Rechner BGB: Fristen online berechnen",
    metaDesc: "Kostenloser Online-Rechner für Kündigungsfristen bei Dienstverträgen, Arbeitsverträgen und B2B-Dauerschuldverhältnissen nach BGB.",
    h1: "Kündigungsfrist-Rechner BGB: Gesetzliche & vertragliche Fristen ermitteln",
    category: "Online-Rechner",
    intro: "Berechnen Sie in Sekunden, bis wann eine Kündigung zugehen muss, um zum gewünschten Termin wirksam zu werden – inklusive Feiertags- und Wochenendberechnung nach § 193 BGB.",
    riskHeadline: "Wichtige Fristenregeln nach BGB",
    riskPoints: [
      "Fristberechnung nach § 187, § 188 BGB: Der Tag des Zugangs zählt nicht mit.",
      "Fällt das Fristende auf einen Samstag, Sonntag oder Feiertag, greift § 193 BGB.",
      "Bei Dauerschuldverhältnissen im B2B sind Kündigungsfristen von mehr als 3 Monaten oft unwirksam."
    ],
    bgbRule: "§ 187 ff. BGB (Fristen & Termine), § 620 ff. BGB.",
    safeDraft: "„Nutzen Sie ClauseLens Studio, um Ihren gesamten Vertrag hochzuladen und Kündigungsfristen automatisch extrahieren zu lassen.“"
  },
  {
    relPath: "tools/verzugszinsen-rechner-b2b.html",
    title: "B2B-Verzugszinsen-Rechner: Zinsen online berechnen",
    metaDesc: "Berechnen Sie Verzugszinsen für gewerbliche Forderungen: 9 Prozentpunkte über dem aktuellen Basiszinssatz der Bundesbank.",
    h1: "B2B-Verzugszinsen-Rechner: Zinsen & 40 € Verzugspauschale berechnen",
    category: "Online-Rechner",
    intro: "Ermitteln Sie tagesgenau die gesetzlichen Verzugszinsen zwischen Unternehmen gemäß § 288 Abs. 2 BGB auf Basis des aktuellen EZB/Bundesbank-Basiszinssatzes.",
    riskHeadline: "Berechnungsgrundlagen im B2B",
    riskPoints: [
      "Aktueller Basiszins + 9 Prozentpunkte für Entgeltforderungen zwischen Kaufleuten.",
      "Zusätzlicher Anspruch auf 40 Euro Pauschale gemäß § 288 Abs. 5 BGB.",
      "Zinseszinsverbot nach § 289 BGB beachten."
    ],
    bgbRule: "§ 288 Abs. 2 BGB, § 247 BGB (Basiszinssatz).",
    safeDraft: "„Verzugszins = Rechnungsbetrag × (Basiszinssatz + 9 %) × Verzugstage / 365 + 40 € Pauschale.“"
  },
  {
    relPath: "tools/agb-risiko-scanner-online.html",
    title: "AGB-Risiko-Scanner online: Verträge sofort prüfen",
    metaDesc: "Kostenloser Sofort-Check für B2B-Verträge: Klauseln einfügen, BGB-Konformität prüfen und Gefahren-Score ermitteln.",
    h1: "AGB-Risiko-Scanner: Kostenlose Vorprüfung für B2B-Verträge",
    category: "Online-Tools",
    intro: "Kopieren Sie verdächtige Vertragsklauseln in unseren Scanner und erhalten Sie eine sofortige Risikoeinschätzung mit Gegenentwurf nach BGB.",
    riskHeadline: "Was prüft der AGB-Scanner?",
    riskPoints: [
      "Automatische Verlängerungsklauseln und unzulässige Fristen.",
      "Einseitige Haftungsausschlüsse und versteckte Verjährungskürzungen.",
      "Überraschende Vertragsstrafenregelungen und Abwerbeverbote."
    ],
    bgbRule: "§ 305–310 BGB.",
    safeDraft: "„Starten Sie direkt im ClauseLens Studio für den vollständigen Batch-Audit ganzer Dokumente.“"
  },
  {
    relPath: "tools/vertragsstrafen-rechner-angemessenheit.html",
    title: "Vertragsstrafen-Rechner: Angemessenheit nach BGB prüfen",
    metaDesc: "Prüfen Sie online, ob eine Vertragsstrafe im Vertrag angemessen oder nach § 307 BGB wegen Sittenwidrigkeit unwirksam ist.",
    h1: "Vertragsstrafen-Rechner: Angemessenheitskontrolle nach § 343 BGB",
    category: "Online-Rechner",
    intro: "Prüfen Sie, ob eine verlangte Vertragsstrafe den vom Bundesgerichtshof gezogenen Grenzen für Verzug oder Pflichtverletzungen entspricht.",
    riskHeadline: "BGH-Kriterien für Vertragsstrafen",
    riskPoints: [
      "Tagespauschalen dürfen in der Regel 0,2 % bis 0,3 % der Auftragssumme nicht übersteigen.",
      "Die Gesamtsumme der Vertragsstrafe muss bei maximal 5 % bis 10 % gedeckelt sein.",
      "Starre Strafen ohne Berücksichtigung des tatsächlichen Verschuldens sind nichtig."
    ],
    bgbRule: "§ 343 BGB, § 307 BGB.",
    safeDraft: "„Eine wirksame Vertragsstrafenklausel erfordert immer eine Höchstgrenze und ein Verschuldenserfordernis.“"
  },
  {
    relPath: "tools/haftungsbegrenzung-rechner-it.html",
    title: "IT-Haftungsbegrenzungs-Rechner: Risiken kalkulieren",
    metaDesc: "Berechnen Sie die rechtssichere Haftungshöchstgrenze für IT-Projekte und Rahmenverträge nach BGH-Rechtsprechung.",
    h1: "IT-Haftungsbegrenzungs-Rechner: Haftungs-Cap ermitteln",
    category: "Online-Rechner",
    intro: "Bestimmen Sie die zulässige Obergrenze (Haftungs-Cap) für Ihr IT-Projekt im Verhältnis zu Projektvolumen und Deckungssumme der Betriebshaftpflicht.",
    riskHeadline: "Faktoren für ein wirksames Haftungs-Cap",
    riskPoints: [
      "Haftungsobergrenze muss in einem angemessenen Verhältnis zum vertragstypischen Risiko stehen.",
      "Ein Cap auf das 1-fache bis 2-fache der Jahresvergütung wird von Gerichten oft als Untergrenze akzeptiert.",
      "Deckungssumme der Haftpflichtversicherung ist ein starkes Indiz für die Angemessenheit."
    ],
    bgbRule: "§ 307 BGB, § 309 Nr. 7 BGB.",
    safeDraft: "„Maximalhaftung = Vertragstypisch vorhersehbarer Schaden (Orientierung an Auftragswert bzw. Jahresvergütung).“"
  },
  {
    relPath: "tools/redline-differenz-pruefer-bgb.html",
    title: "Redline & Differenz-Prüfer: Vertragsversionen vergleichen",
    metaDesc: "Vergleichen Sie Erstentwurf und Gegenentwurf nach Verhandlungen: Erkennen Sie Risikodeltas und veränderte AGB-Klauseln.",
    h1: "Redline & Differenz-Prüfer: Risikovergleich nach Nachverhandlungen",
    category: "Online-Tools",
    intro: "Vergleichen Sie zwei Fassungen eines Vertrags und sehen Sie sofort, welche juristischen Risiken Ihr Verhandlungspartner entschärft oder neu eingefügt hat.",
    riskHeadline: "Vorteile der Risikodelta-Analyse",
    riskPoints: [
      "Erkennt auch subtile Textänderungen (z. B. Streichung von Wörtern wie 'vorsätzlich').",
      "Berechnet das Risikodelta vor und nach der Verhandlungsrunde.",
      "Exportiert direkt verhandlungsfertige Gegenentwürfe im Markdown- und HTML-Format."
    ],
    bgbRule: "§ 305 ff. BGB.",
    safeDraft: "„Nutzen Sie den integrierten CLI-Befehl `node bin/clauselens.js compare v1.docx v2.docx` für den sofortigen Versionsvergleich.“"
  }
];

console.log(`Generating ${PAGES.length} programmatic SEO landing pages...`);

function renderPage(p) {
  // Determine relative root path based on directory depth
  const depth = p.relPath.split("/").length - 1;
  const relRoot = depth === 1 ? "../" : "../../";

  const riskListHtml = p.riskPoints.map(pt => `<li>${pt}</li>`).join("\n              ");

  return `<!doctype html>
<html lang="de">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>${p.title} &middot; ClauseLens</title>
  <meta name="description" content="${p.metaDesc}">
  <link rel="canonical" href="https://clause.kellersystems.dev/${p.relPath}">
  <link rel="icon" href="${relRoot}assets/icon.svg" type="image/svg+xml">
  <meta property="og:title" content="${p.title}">
  <meta property="og:description" content="${p.metaDesc}">
  <meta property="og:type" content="article">
  <meta property="og:url" content="https://clause.kellersystems.dev/${p.relPath}">
  <meta property="og:site_name" content="ClauseLens">
  <link rel="preconnect" href="https://fonts.googleapis.com">
  <link rel="preconnect" href="https://fonts.gstatic.com" crossorigin>
  <link href="https://fonts.googleapis.com/css2?family=IBM+Plex+Mono:wght@400;500;600&family=Inter:wght@400;500;600;700&family=Source+Serif+4:opsz,wght@8..60,400;8..60,600&display=swap" rel="stylesheet">
  <script type="application/ld+json">
    {"@context":"https://schema.org","@type":"Article","headline":"${p.title}","description":"${p.metaDesc}","author":{"@type":"Organization","name":"ClauseLens / Keller Systems"},"publisher":{"@type":"Organization","name":"Keller Systems","url":"https://kellersystems.dev"}}
  </script>
  <style>
    ${baseCss}
    .badge-cat {
      display: inline-block;
      font-family: 'IBM Plex Mono', monospace;
      font-size: 11px;
      text-transform: uppercase;
      letter-spacing: 0.08em;
      background: var(--accent-bg);
      color: var(--accent);
      padding: 4px 10px;
      border-radius: 4px;
      font-weight: 600;
      margin-bottom: 12px;
    }
    .cta-box {
      background: #ffffff;
      border: 1px solid var(--rule);
      border-left: 4px solid var(--accent);
      padding: 24px;
      border-radius: 8px;
      margin: 32px 0;
    }
    .cta-box h3 {
      font-family: 'Source Serif 4', Georgia, serif;
      font-size: 1.35rem;
      color: var(--ink);
      margin-bottom: 8px;
    }
    .cta-box p {
      color: var(--body);
      margin-bottom: 16px;
      font-size: 15px;
    }
    .btn-action {
      display: inline-flex;
      align-items: center;
      gap: 8px;
      background: var(--accent);
      color: #ffffff;
      font-family: 'Inter', system-ui, sans-serif;
      font-weight: 600;
      font-size: 14px;
      padding: 10px 20px;
      border-radius: 6px;
      text-decoration: none;
      transition: background 0.15s ease;
    }
    .btn-action:hover {
      background: #1e3a5f;
    }
    .redline-box {
      background: var(--green-bg);
      border: 1px solid var(--green);
      border-radius: 6px;
      padding: 16px 20px;
      margin: 20px 0;
      font-size: 14.5px;
      line-height: 1.6;
      color: #1a3e20;
    }
    .redline-box b {
      color: #1a3e20;
      display: block;
      margin-bottom: 6px;
      font-family: 'IBM Plex Mono', monospace;
      font-size: 12px;
      text-transform: uppercase;
    }
    .rule-tag {
      font-family: 'IBM Plex Mono', monospace;
      font-size: 12px;
      color: var(--dim);
      background: rgba(0,0,0,0.04);
      padding: 3px 8px;
      border-radius: 4px;
      display: inline-block;
      margin-top: 8px;
    }
  </style>
</head>
<body>
  <header class="site-nav">
    <div class="nav-inner">
      <a href="${relRoot}" class="brand-badge">ClauseLens<span class="tag">Studio</span></a>
      <div class="nav-links">
        <a href="${relRoot}studio.html" class="nav-link">Studio öffnen</a>
        <a href="${relRoot}klauseln/" class="nav-link">Lexikon</a>
        <a href="${relRoot}pricing.html" class="nav-link">Preise</a>
      </div>
    </div>
  </header>

  <main class="wrap" style="padding-top: 40px; padding-bottom: 60px;">
    <span class="badge-cat">${p.category}</span>
    <h1 style="font-family: 'Source Serif 4', Georgia, serif; font-size: clamp(1.8rem, 3.5vw, 2.5rem); color: var(--ink); line-height: 1.25; margin-bottom: 18px;">${p.h1}</h1>
    
    <p style="font-size: 17px; line-height: 1.65; color: var(--ink); margin-bottom: 28px;">
      ${p.intro}
    </p>

    <div class="cta-box">
      <h3>Diesen Vertrag jetzt lokal & DSGVO-konform prüfen</h3>
      <p>Laden Sie Ihr Vertragsdokument als PDF oder DOCX in das ClauseLens Studio. Die Engine prüft alle Klauseln in Millisekunden lokal im Browser – 100% offline ohne Datenübertragung an Cloud-Server.</p>
      <a href="${relRoot}studio.html" class="btn-action">Im ClauseLens Studio prüfen &rarr;</a>
    </div>

    <section style="margin: 40px 0;">
      <h2 style="font-family: 'Source Serif 4', Georgia, serif; font-size: 1.5rem; color: var(--ink); margin-bottom: 14px;">${p.riskHeadline}</h2>
      <ul style="margin-left: 20px; line-height: 1.8; color: var(--body);">
        ${riskListHtml}
      </ul>
      <div class="rule-tag">Rechtsgrundlage: ${p.bgbRule}</div>
    </section>

    <section style="margin: 40px 0;">
      <h2 style="font-family: 'Source Serif 4', Georgia, serif; font-size: 1.5rem; color: var(--ink); margin-bottom: 12px;">Rechtssicherer Gegenentwurf (Musterklausel)</h2>
      <p style="color: var(--body); font-size: 15px; margin-bottom: 10px;">Nutzen Sie diese geprüfte Redline-Klausel für die Nachverhandlung:</p>
      <div class="redline-box">
        <b>Empfohlene Alternativklausel:</b>
        ${p.safeDraft}
      </div>
    </section>

    <section style="margin: 50px 0; border-top: 1px solid var(--rule); padding-top: 30px;">
      <h3 style="font-family: 'Source Serif 4', Georgia, serif; font-size: 1.3rem; color: var(--ink); margin-bottom: 12px;">Automatisierte Vertragsanalyse mit ClauseLens</h3>
      <p style="color: var(--body); font-size: 15px; line-height: 1.65; margin-bottom: 20px;">
        ClauseLens wurde speziell für deutsche IT-Unternehmen, Freelancer, Agenturen und Kanzleien entwickelt. Unsere deterministische BGB-Regelmaschine erkennt über 47 typische Klauselfallen mit 0 % Halluzinationen.
      </p>
      <div style="display: flex; gap: 12px; flex-wrap: wrap;">
        <a href="${relRoot}studio.html" class="btn-action">Kostenlos testen</a>
        <a href="${relRoot}pricing.html" style="display: inline-flex; align-items: center; padding: 10px 18px; border: 1px solid var(--rule); border-radius: 6px; color: var(--body); text-decoration: none; font-weight: 500;">Preise ansehen</a>
      </div>
    </section>
  </main>

  <footer style="background: var(--card); border-top: 1px solid var(--rule); padding: 30px 20px; text-align: center; font-size: 13px; color: var(--dim);">
    <div class="wrap">
      <p>&copy; 2026 ClauseLens &bull; Keller Systems. Alle Rechte vorbehalten.</p>
      <p style="margin-top: 6px;">
        <a href="${relRoot}impressum.html" style="color: var(--dim); text-decoration: none;">Impressum</a> &bull;
        <a href="${relRoot}privacy.html" style="color: var(--dim); text-decoration: none;">Datenschutz</a> &bull;
        <a href="${relRoot}terms.html" style="color: var(--dim); text-decoration: none;">AGB</a> &bull;
        <a href="${relRoot}klauseln/" style="color: var(--dim); text-decoration: none;">Klausel-Lexikon</a>
      </p>
    </div>
  </footer>
</body>
</html>`;
}

let count = 0;
for (const p of PAGES) {
  const fullPath = join(ROOT_DIR, p.relPath);
  const dir = dirname(fullPath);
  if (!existsSync(dir)) {
    mkdirSync(dir, { recursive: true });
  }
  writeFileSync(fullPath, renderPage(p), "utf8");
  count++;
}

console.log(`Successfully generated ${count} HTML pages.`);
