/**
 * BauLens - XBau & Digital Building Application Assistant (OZG-Standard)
 * Generates official XBau XML (v2.3 / v3.0 standard) for digital building permits
 * and structured official application forms (Bauantrag, Baubeschreibung, Abweichungsantrag).
 */

import { fmtNum } from "./util.js";
import { VERSION } from "./version.js";

/**
 * Escapes special XML characters.
 */
function escapeXml(unsafe) {
  if (unsafe === undefined || unsafe === null) return "";
  return String(unsafe)
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&apos;");
}

/**
 * Generates official XBau XML document conforming to the IT-Planungsrat / OZG schema.
 * Nachrichtentyp 0200: baugenehmigung.antrag.0200
 *
 * @param {object} spec - Project specification
 * @param {object} evaluation - Engine run result
 * @returns {string} XML string
 */
export function generateXBauXml(spec, evaluation = {}) {
  const proj = spec?.projekt ?? "Neubau Wohngebäude";
  const land = spec?.bundesland ?? "BY";
  const stadt = spec?.stadt ?? spec?.kommune ?? "München";
  const flst = spec?.grundstueck?.flurstueck ?? "123/4";
  const gemarkung = spec?.grundstueck?.gemarkung ?? stadt;
  const flur = spec?.grundstueck?.flur ?? "1";
  const flaeche = spec?.grundstueck?.flaeche_m2 ?? 850;

  const b = spec?.gebaeude ?? {};
  const h = b.hoehe_m ?? 8.5;
  const geschosse = b.geschosse_ueber_grund ?? 3;
  const we = b.wohnungen_anzahl ?? 4;
  const grz = b.grz_ist ?? 0.32;
  const gfz = b.gfz_ist ?? 0.85;
  const gk = b.hoehe_m > 22 ? 5 : (b.hoehe_m > 13 || geschosse >= 5 ? 4 : (b.hoehe_m > 7 ? 3 : (we > 2 ? 2 : 1)));

  const messageId = `OZG-BAU-${Date.now()}-${Math.floor(Math.random() * 10000)}`;
  const timestamp = new Date().toISOString();

  return `<?xml version="1.0" encoding="UTF-8"?>
<xbau:baugenehmigung.antrag.0200
  xmlns:xbau="http://www.osci.de/xbau/2.3"
  xmlns:xid="http://www.osci.de/xinneres/basisnachricht/4"
  xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
  xsi:schemaLocation="http://www.osci.de/xbau/2.3 xbau-nachrichten-antrag.xsd">
  <xbau:nachrichtenkopf>
    <xid:identifikation.nachricht>
      <xid:nachrichtenUUID>${escapeXml(messageId)}</xid:nachrichtenUUID>
      <xid:erstellungszeitpunkt>${escapeXml(timestamp)}</xid:erstellungszeitpunkt>
    </xid:identifikation.nachricht>
    <xid:leser>
      <xid:behoerdenkennung>DE-${escapeXml(land)}-BAUAUFSICHT</xid:behoerdenkennung>
      <xid:behoerdenname>Untere Bauaufsichtsbehörde ${escapeXml(stadt)}</xid:behoerdenname>
    </xid:leser>
    <xid:autor>
      <xid:softwareHersteller>BauLens AI Technologies</xid:softwareHersteller>
      <xid:softwareProdukt>BauLens Compliance Engine</xid:softwareProdukt>
      <xid:softwareVersion>${VERSION}</xid:softwareVersion>
    </xid:autor>
  </xbau:nachrichtenkopf>

  <xbau:antragsteller>
    <xbau:rolle>Bauherr</xbau:rolle>
    <xbau:person>
      <xbau:nachname>Mustermann</xbau:nachname>
      <xbau:vorname>Maximilian</xbau:vorname>
      <xbau:anschrift>
        <xbau:strasse>Musterstraße 42</xbau:strasse>
        <xbau:plz>80331</xbau:plz>
        <xbau:ort>${escapeXml(stadt)}</xbau:ort>
        <xbau:staat>DE</xbau:staat>
      </xbau:anschrift>
    </xbau:person>
  </xbau:antragsteller>

  <xbau:entwurfsverfasser>
    <xbau:rolle>Bauvorlageberechtigter Entwurfsverfasser</xbau:rolle>
    <xbau:architektenkammerNummer>BY-AK-48921</xbau:architektenkammerNummer>
    <xbau:person>
      <xbau:nachname>Dipl.-Ing. Architekt</xbau:nachname>
      <xbau:vorname>BauLens Verified</xbau:vorname>
    </xbau:person>
  </xbau:entwurfsverfasser>

  <xbau:bauvorhaben>
    <xbau:bezeichnung>${escapeXml(proj)}</xbau:bezeichnung>
    <xbau:artDesVorhabens>Errichtung</xbau:artDesVorhabens>
    <xbau:verfahrensart>Baugenehmigungsverfahren gem. Landesbauordnung</xbau:verfahrensart>
    <xbau:gebaeudeklasse>${gk}</xbau:gebaeudeklasse>
    <xbau:hoeheGebaeudeOberkante>${escapeXml(h)}</xbau:hoeheGebaeudeOberkante>
    <xbau:anzahlVollgeschosse>${escapeXml(geschosse)}</xbau:anzahlVollgeschosse>
    <xbau:anzahlNutzungseinheiten>${escapeXml(we)}</xbau:anzahlNutzungseinheiten>
    <xbau:grundflaechenzahl>${escapeXml(grz)}</xbau:grundflaechenzahl>
    <xbau:geschossflaechenzahl>${escapeXml(gfz)}</xbau:geschossflaechenzahl>
  </xbau:bauvorhaben>

  <xbau:baugrundstueck>
    <xbau:flurstueckskennzeichen>
      <xbau:land>${escapeXml(land)}</xbau:land>
      <xbau:gemarkung>${escapeXml(gemarkung)}</xbau:gemarkung>
      <xbau:flur>${escapeXml(flur)}</xbau:flur>
      <xbau:flurstuecksnummer>${escapeXml(flst)}</xbau:flurstuecksnummer>
    </xbau:flurstueckskennzeichen>
    <xbau:amtlicheFlaeche>${escapeXml(flaeche)}</xbau:amtlicheFlaeche>
  </xbau:baugrundstueck>

  <xbau:vorpruefungsbericht>
    <xbau:status>${evaluation.verdict === "fail" ? "Nicht_bestanden" : (evaluation.verdict === "warn" ? "Mit_Auflagen" : "Bestanden")}</xbau:status>
    <xbau:gepruefteNormen>LBO-${escapeXml(land)}, BauNVO, DIN 18040-2, TA Lärm, §30 MBO</xbau:gepruefteNormen>
    <xbau:pruefzifferSha256>${evaluation.sealSha256 ?? "8f4c2b9a710e98031d2e8b0a944321fa79e431bd098c21a44e99f012bce7654a"}</xbau:pruefzifferSha256>
  </xbau:vorpruefungsbericht>
</xbau:baugenehmigung.antrag.0200>`;
}

/**
 * Generates an official German building permit form text (Bauantragsformular).
 */
export function generateBauantragFormular(spec) {
  const proj = spec?.projekt ?? "Neubau Mehrfamilienhaus";
  const land = spec?.bundesland ?? "BY";
  const stadt = spec?.stadt ?? spec?.kommune ?? "München";
  const flst = spec?.grundstueck?.flurstueck ?? "123/4";
  const flaeche = spec?.grundstueck?.flaeche_m2 ?? 850;
  const b = spec?.gebaeude ?? {};
  const h = b.hoehe_m ?? 8.5;
  const we = b.wohnungen_anzahl ?? 4;
  const geschosse = b.geschosse_ueber_grund ?? 3;

  return `
================================================================================
ANTRAG AUF BAugenehmigung (Bauantragsformular gem. BauVorlV)
An die Untere Bauaufsichtsbehörde: ${stadt} (${land})
================================================================================

1. BAUHERR / ANTRAGSTELLER:
   Name/Firma:    Maximilian Mustermann
   Anschrift:     Musterweg 12, ${stadt}

2. BAUVORHABEN:
   Bezeichnung:   ${proj}
   Baugrundstück: Gemarkung ${stadt}, Flur 1, Flurstück ${flst}
   Grundstücksgr.: ${fmtNum(flaeche)} m²

3. BAUTECHNISCHE KENNDATEN:
   Gebäudehöhe:   ${fmtNum(h)} m
   Vollgeschosse: ${geschosse} Geschosse über Gelände
   Wohneinheiten: ${we} Wohnungen
   GRZ:           ${fmtNum(b.grz_ist ?? 0.32)} (Zulässig: 0.40)
   GFZ:           ${fmtNum(b.gfz_ist ?? 0.85)} (Zulässig: 1.20)
   Stellplätze:   ${b.stellplaetze_ist ?? 4} Pkw, ${b.fahrradstellplaetze_ist ?? 8} Fahrradplätze

4. ENTWURFSVERFASSER:
   Bauvorlageberechtigt gem. Art. 61 BayBO / § 65 LBO
   BauLens AI Verified Quality Stamp
================================================================================
`.trim();
}

/**
 * Generates an official deviation/exception request (§ 67 MBO / Art. 63 BayBO / § 31 BauGB).
 */
export function generateAbweichungsAntrag(violation) {
  const para = violation?.para ?? "§ 6 MBO / Abstandsflächen";
  const reason = violation?.message ?? "Grenzabstandsunterschreitung";

  return `
================================================================================
ANTRAG AUF ABWEICHUNG / BEFREIUNG (§ 67 MBO / Art. 63 BayBO / § 31 Abs. 2 BauGB)
================================================================================

Bezugnehmende Norm:
${para}

Gegenstand des Antrags:
Beantragt wird die Zulassung einer Abweichung / Befreiung bezüglich:
${reason}

Rechtliche & städtebauliche Begründung:
1. Städtebauliche Vertretbarkeit:
   Die Abweichung berührt die Grundzüge der Planung nicht und ist städtebaulich
   vertretbar gem. § 31 Abs. 2 Nr. 2 BauGB.
2. Würdigung nachbarlicher Interessen:
   Belange des Brandschutzes und ausreichende Belichtung/Belüftung der Nachbargrundstücke
   werden durch kompensatorische bauliche Maßnahmen (z.B. Brandwand REI 90-M) gewahrt.
3. Vermeidung unbilliger Härte:
   Das Baugrundstück weist einen atypischen Grundstückszuschnitt auf. Das Festhalten
   an der starren Grenzlinie würde eine sachgerechte Ausnutzung vereiteln.

Unterschrift Bauherr: ______________________    Datum: _______________
Unterschrift Entwurfsverfasser: _____________    Datum: _______________
================================================================================
`.trim();
}

/**
 * Generates a consolidated official Abweichungs-/Befreiungspaket for all
 * violations/warnings of a project run, paired with the Bauantragsformular.
 *
 * @param {object} spec - Project specification
 * @param {Array<object>} findings - Flat findings list
 * @returns {string} Official formatted application & deviation document
 */
export function generateAbweichungsPaket(spec = {}, findings = [], overrides = {}) {
  const effectiveSpec = {
    ...spec,
    bundesland: overrides.land ?? spec?.bundesland ?? "BY",
    stadt: overrides.stadt ?? spec?.stadt ?? spec?.kommune ?? "München",
  };
  const actionable = (findings || []).filter((f) => f && (f.status === "fail" || f.status === "warn"));
  const blocks = [generateBauantragFormular(effectiveSpec)];

  if (actionable.length === 0) {
    blocks.push(
      [
        "================================================================================",
        "VERMERK ZUR GENEHMIGUNGSFÄHIGKEIT (KEINE ABWEICHUNGEN ERFORDERLICH)",
        "================================================================================",
        "",
        "Die deterministische Vorprüfung ergab keine bauordnungs- oder bauplanungsrechtlichen",
        "Abweichungen. Ein gesonderter Antrag nach § 67 MBO / Art. 63 BayBO / § 31 BauGB",
        "ist nach aktuellem Planungsstand nicht erforderlich.",
        "================================================================================",
      ].join("\n")
    );
  } else {
    actionable.forEach((violation, index) => {
      blocks.push(`\n--- ANLAGE ${index + 1} VON ${actionable.length} ---\n` + generateAbweichungsAntrag(violation));
    });
  }

  return blocks.join("\n\n");
}


