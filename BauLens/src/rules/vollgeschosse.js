import { fmtNum } from "../util.js";

const MAX_MITTLERE_GESCHOSSHOEHE = 3.5;

export default {
  id: "vollgeschosse",
  para: "§16 BauNVO",
  title: "Plausibilität: mittlere Geschosshöhe vs. Gebäudehöhe (Konsistenz)",
  evaluate(spec) {
    const h = spec?.gebaeude?.hoehe_m;
    const gug = spec?.gebaeude?.geschosse_ueber_grund;

    if (!Number.isFinite(h) || !Number.isInteger(gug) || gug <= 0) {
      return [];
    }

    const mittel = h / gug;
    const findings = [];

    if (mittel > MAX_MITTLERE_GESCHOSSHOEHE + 1e-9) {
      findings.push({
        status: "warn",
        message: `Mittlere Geschosshöhe ${fmtNum(mittel)} m (Höhe ${fmtNum(h)} m / ${gug} Vollgeschosse) überschreitet ${fmtNum(MAX_MITTLERE_GESCHOSSHOEHE)} m – Widerspruch zwischen Höhe und Vollgeschossen; prüfen, ob Keller-/Staffelgeschosse korrekt erfasst sind (Abgrenzung Vollgeschoss nach §16 BauNVO).`,
      });
    } else {
      findings.push({
        status: "ok",
        message: `Mittlere Geschosshöhe ${fmtNum(mittel)} m ist plausibel für ${gug} Vollgeschosse.`,
      });
    }

    // Dachgeschoss-Vollgeschossprüfung (2/3- bzw. 3/4-Regel nach LBO / BauNVO)
    const dg = spec?.gebaeude?.dachgeschoss;
    if (dg?.vorhanden && Number.isFinite(dg.flaeche_ueber_2_30m_m2) && Number.isFinite(dg.flaeche_unterliegend_m2) && dg.flaeche_unterliegend_m2 > 0) {
      const anteil = dg.flaeche_ueber_2_30m_m2 / dg.flaeche_unterliegend_m2;
      const prozent = Math.round(anteil * 100);
      // In fast allen LBOs gilt: >= 2/3 (66.7%) oder >= 3/4 (75%) gilt das Dachgeschoss als Vollgeschoss
      const schwelle = 2 / 3;
      const istVollgeschoss = anteil >= schwelle - 1e-9;

      if (istVollgeschoss) {
        findings.push({
          status: "warn",
          message: `Dachgeschoss gilt als Vollgeschoss: Fläche mit lichter Höhe >= 2,30 m (${fmtNum(dg.flaeche_ueber_2_30m_m2)} m²) erreicht ${prozent} % der Grundfläche des darunterliegenden Geschosses (${fmtNum(dg.flaeche_unterliegend_m2)} m² >= 67 % Schwelle nach LBO). Bei Vollgeschoss-Höchstgrenzen im B-Plan beachten!`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Dachgeschoss ist KEIN Vollgeschoss: Fläche mit lichter Höhe >= 2,30 m (${fmtNum(dg.flaeche_ueber_2_30m_m2)} m²) beträgt nur ${prozent} % des darunterliegenden Geschosses (< 67 % Schwelle).`,
        });
      }
    }

    return findings;
  },
};
