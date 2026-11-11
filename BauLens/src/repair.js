/**
 * BauLens - Compliance Repair Engine
 * Suggests concrete architectural modifications and legal remedies when violations occur:
 * - Option A: Spatial Translation (Shift building footprint into available setback buffer)
 * - Option B: Height / Geometry Optimization (Trim building height / adjust roof slope)
 * - Option C: Legal Exemption Filing (§ 67 MBO / Art. 63 BayBO / § 31 BauGB deviation draft)
 */

import { fmtNum, isNum } from "./util.js";

function applyPatchToSpec(spec, patch) {
  const clone = JSON.parse(JSON.stringify(spec ?? {}));
  if (!patch || typeof patch !== "object") return clone;
  for (const [dotted, val] of Object.entries(patch)) {
    const parts = dotted.split(".");
    let cur = clone;
    for (let i = 0; i < parts.length - 1; i++) {
      const k = parts[i];
      if (!cur[k] || typeof cur[k] !== "object") cur[k] = {};
      cur = cur[k];
    }
    cur[parts[parts.length - 1]] = val;
  }
  return clone;
}

/**
 * Evaluates findings and generates proactive repair suggestions.
 * @param {object} spec - Project spec
 * @param {Array<object>} findings - Rule evaluation findings
 * @param {object} profile - Active profile (e.g. MBO, BY, BW)
 * @returns {Array<object>} Suggestions per violation
 */
export function generateComplianceRepairs(spec, findings, profile = {}) {
  const suggestions = [];
  const seen = new Set();
  const b = spec?.gebaeude ?? {};
  const grenz = spec?.grundstueck?.grenzabstaende ?? {};
  const factor = profile.factor ?? 0.5;
  const minM = profile.minM ?? 3.0;

  function pushSuggestion(item) {
    const key = `${item.ruleId}:${item.type}:${item.title}`;
    if (seen.has(key)) return;
    seen.add(key);
    const action = item.action ?? item.description ?? "";
    const description = item.description ?? item.action ?? "";
    const patchedSpec = item.patch ? applyPatchToSpec(spec, item.patch) : null;
    suggestions.push({
      costImpact: item.costImpact ?? (item.type === "legal_remedy" ? "niedrig" : "mittel"),
      confidence: item.confidence ?? 0.9,
      ...item,
      action,
      description,
      ...(patchedSpec ? { patchedSpec } : {}),
    });
  }

  for (const f of findings) {
    if (f.status !== "fail" && f.status !== "warn") continue;

    // 1. Abstandsflächen-Defizit
    if (f.ruleId === "abstandsflaeche" || f.message.includes("Abstandsfläche") || f.message.includes("Grenzabstand")) {
      const h = b.hoehe_m ?? 8.5;
      const reqM = Math.max(minM, h * factor);

      // Analyze opposite side buffers
      // Ost vs West
      if (isNum(grenz.osten) && grenz.osten < reqM && isNum(grenz.westen)) {
        const deficitM = +(reqM - grenz.osten).toFixed(2);
        const westBuffer = grenz.westen - reqM;
        if (westBuffer >= deficitM) {
          pushSuggestion({
            type: "spatial_shift",
            ruleId: "abstandsflaeche",
            title: `Lösung A: Verschiebung des Baukörpers um ${fmtNum(deficitM)} m nach Westen`,
            delta: `+${fmtNum(deficitM)} m Ost`,
            description: `Grenzabstand Ost (${fmtNum(grenz.osten)} m) unterschreitet Soll (${fmtNum(reqM)} m). Auf der Westseite existiert ein Überschuss-Puffer von ${fmtNum(grenz.westen)} m. Ein Verschieben um ${fmtNum(deficitM)} m stellt die Konformität ohne Flächenverlust her.`,
            patch: { "grundstueck.grenzabstaende.osten": +(grenz.osten + deficitM).toFixed(2), "grundstueck.grenzabstaende.westen": +(grenz.westen - deficitM).toFixed(2) },
          });
        }
      }

      // Nord vs Süd
      if (isNum(grenz.norden) && grenz.norden < reqM && isNum(grenz.sueden)) {
        const deficitM = +(reqM - grenz.norden).toFixed(2);
        const suedBuffer = grenz.sueden - reqM;
        if (suedBuffer >= deficitM) {
          pushSuggestion({
            type: "spatial_shift",
            ruleId: "abstandsflaeche",
            title: `Lösung A: Verschiebung des Baukörpers um ${fmtNum(deficitM)} m nach Süden`,
            delta: `+${fmtNum(deficitM)} m Nord`,
            description: `Grenzabstand Nord (${fmtNum(grenz.norden)} m) unterschreitet Soll (${fmtNum(reqM)} m). Auf der Südseite existiert ein Puffer von ${fmtNum(grenz.sueden)} m. Ein Verschieben um ${fmtNum(deficitM)} m heilt den Grenzabstand.`,
            patch: { "grundstueck.grenzabstaende.norden": +(grenz.norden + deficitM).toFixed(2), "grundstueck.grenzabstaende.sueden": +(grenz.sueden - deficitM).toFixed(2) },
          });
        }
      }

      // Direct setback widening if any side is below reqM
      const setPatch = {};
      for (const dir of ["norden", "osten", "sueden", "westen"]) {
        if (isNum(grenz[dir]) && grenz[dir] < reqM) {
          setPatch[`grundstueck.grenzabstaende.${dir}`] = +reqM.toFixed(2);
        }
      }
      if (Object.keys(setPatch).length > 0) {
        pushSuggestion({
          type: "setback_widen",
          ruleId: "abstandsflaeche",
          title: `Grenzabstand auf Sollmaß (${fmtNum(reqM)} m) erweitern`,
          delta: `Abstand ≥ ${fmtNum(reqM)} m`,
          description: `Baukörper von den betroffenen Grundstücksgrenzen auf mindestens ${fmtNum(reqM)} m abrücken, um die Abstandsfläche auf eigenem Grundstück nachzuweisen.`,
          patch: setPatch,
        });
      }

      // Lösung B: Höhenreduktion (wenn Verschiebung nicht ausreicht)
      const maxAllowedH = Math.min(grenz.osten ?? Infinity, grenz.westen ?? Infinity, grenz.norden ?? Infinity, grenz.sueden ?? Infinity) / factor;
      if (maxAllowedH >= minM / factor && maxAllowedH < h) {
        const deltaH = +(h - maxAllowedH).toFixed(2);
        pushSuggestion({
          type: "height_trim",
          ruleId: "abstandsflaeche",
          title: `Lösung B: Reduzierung der Wandhöhe um ${fmtNum(deltaH)} m`,
          delta: `H -${fmtNum(deltaH)} m`,
          description: `Absenkung der Trauf-/Wandhöhe von ${fmtNum(h)} m auf ${fmtNum(maxAllowedH)} m (z.B. Kniestockanpassung oder Neigungsverflachung) reduziert die erforderliche Abstandsfläche exakt auf die Grundstücksgrenze.`,
          patch: { "gebaeude.hoehe_m": +maxAllowedH.toFixed(2) },
        });
      }

      // Lösung C: Juristischer Abweichungsantrag
      pushSuggestion({
        type: "legal_remedy",
        ruleId: "abstandsflaeche",
        title: "Lösung C: Antrag auf Abweichung gem. § 67 MBO / Art. 63 BayBO",
        delta: "§ 67 MBO / Baulast",
        description: "Beantragung einer baurechtlichen Abweichung bei der Baubehörde aufgrund atypischer Grundstückssituation. Nachbarliche Würdigung erfolgt über nachgewiesene Belichtungssimulation oder Eintragung einer Abstandsflächenbaulast.",
      });
    }

    // 2. GRZ Überschreitung
    if (f.ruleId === "grz" || f.message.includes("GRZ")) {
      const grzMax = +(spec?.bplan?.grz_max ?? spec?.einstellungen?.grz_limit ?? 0.4).toFixed(2);
      pushSuggestion({
        type: "grz_trim",
        ruleId: "grz",
        title: `Überbaute Grundfläche auf GRZ ${fmtNum(grzMax)} reduzieren`,
        delta: `GRZ → ${fmtNum(grzMax)}`,
        description: `Reduktion der versiegelten Gebäudegrundfläche bzw. Entsiegelung von Nebenanlagen auf den zulässigen GRZ-Richtwert von ${fmtNum(grzMax)} (§ 19 BauNVO).`,
        patch: { "gebaeude.grz_ist": grzMax },
      });
    }

    // 3. GFZ Überschreitung
    if (f.ruleId === "gfz" || f.message.includes("GFZ")) {
      const gfzIst = b.gfz_ist ?? 1.3;
      const gfzMax = +(spec?.bplan?.gfz_max ?? spec?.einstellungen?.gfz_limit ?? 1.2).toFixed(2);
      const excess = Math.max(0.05, +(gfzIst - gfzMax).toFixed(2));
      pushSuggestion({
        type: "usage_trim",
        ruleId: "gfz",
        title: `Lösung A: Umwandlung von Geschossfläche (${fmtNum(excess)} GFZ-Punkte)`,
        delta: `GFZ → ${fmtNum(gfzMax)}`,
        description: `Umwandlung des obersten Vollgeschosses in ein Nicht-Vollgeschoss (Staffelgeschoss mit Grundfläche < 2/3 des darunterliegenden Geschosses) zur Einhaltung der GFZ von ${fmtNum(gfzMax)}.`,
        patch: { "gebaeude.gfz_ist": gfzMax },
      });
      pushSuggestion({
        type: "legal_remedy",
        ruleId: "gfz",
        title: "Lösung B: Städtebauliche Befreiung nach § 31 Abs. 2 BauGB",
        delta: "§ 31 Abs. 2 BauGB",
        description: `Befreiungsantrag von der GFZ-Höchstgrenze gem. § 31 Abs. 2 Nr. 2 BauGB (städtebauliche Vertretbarkeit bei Schaffung von bezahlbarem Wohnraum).`,
      });
    }

    // 4. Rettungswege / Treppenraum
    if (f.ruleId === "rettungswege") {
      pushSuggestion({
        type: "safety_upgrade",
        ruleId: "rettungswege",
        title: "Notwendigen Treppenraum & 2. Rettungsweg vorsehen",
        delta: "§ 33 / § 35 MBO",
        description: "Ausbildung eines notwendigen Treppenraums sowie Nachweis des zweiten Rettungswegs (z.B. Sicherheitstreppenraum oder anleiterbare Stelle für die Feuerwehr).",
        patch: { "gebaeude.treppenraum_vorhanden": true, "gebaeude.zweiter_rettungsweg_vorhanden": true },
      });
    }

    // 5. Stellplätze
    if (f.ruleId === "stellplaetze") {
      const we = Number(b.wohnungen_anzahl) || 4;
      pushSuggestion({
        type: "parking_add",
        ruleId: "stellplaetze",
        title: `Stellplatznachweis (${we} Pkw / ${we * 2} Fahrradplätze) ergänzen`,
        delta: `${we} Pkw · ${we * 2} Rad`,
        description: "Herstellung der nach kommunaler Satzung erforderlichen Stellplätze auf dem Baugrundstück, über ÖPNV-Mobilitätskonzept oder Stellplatzablöse.",
        patch: { "gebaeude.stellplaetze_pkw": we, "gebaeude.stellplaetze_fahrrad": we * 2 },
      });
    }

    // 6. Aufzugspflicht
    if (f.ruleId === "barrierefreiheit" && f.message.includes("Aufzug")) {
      pushSuggestion({
        type: "equipment_add",
        ruleId: "barrierefreiheit",
        title: "Lösung A: Aufzugsschacht im Treppenhauskern nachrüsten",
        delta: "DIN 18040-2",
        description: "Standardisierter Seilaufzug (Mindestkabinenmaß 1,10 m x 2,10 m) erfüllt DIN 18040-2 und OIB RL 4.",
        patch: { "gebaeude.aufzug_vorhanden": true },
      });
    }

    // 7. Wärmepumpen Grenzabstand
    if (f.ruleId === "schallschutz" && f.message.includes("Wärmepumpe")) {
      pushSuggestion({
        type: "sound_proofing",
        ruleId: "schallschutz",
        title: "Lösung A: Schallschutzhaube mit 8-10 dB(A) Dämpfung installieren",
        delta: "-10 dB(A)",
        description: "Durch eine zertifizierte Schalldämmhaube wird der Immissionsrichtwert von 35 dB(A) nachts auch bei geringerem Grenzabstand eingehalten.",
      });
    }
  }

  return suggestions;
}

