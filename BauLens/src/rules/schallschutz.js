import { fmtNum, isNum } from "../util.js";

export const MIN_WP_BOUNDARY_DIST_M = 3.0;

export default {
  id: "schallschutz",
  para: "§15 MBO / DIN 4109 / TA Lärm",
  title: "Schallschutz Außenbauteile & Nachbarschutz Haustechnik (TA Lärm / DIN 4109)",
  evaluate(spec, ctx = {}) {
    const b = spec?.gebaeude ?? {};
    const hasWP = b.waermepumpe_aussen_vorhanden;
    const wpDist = b.abstand_waermepumpe_grenze_m;
    const lpb = b.schallschutz_laermpegelbereich;

    const findings = [];

    // 1. Nachbarschutz Außenaufstellung Luft-Wasser-Wärmepumpen (TA Lärm Richtwert nachts 35-40 dB(A) im reinen/allgem. Wohngebiet)
    if (hasWP === true) {
      if (isNum(wpDist)) {
        if (wpDist < MIN_WP_BOUNDARY_DIST_M) {
          findings.push({
            status: "warn",
            message: `Außen-Wärmepumpe mit nur ${fmtNum(wpDist)} m Grenzabstand unterschreitet den TA-Lärm-Empfehlungsabstand von ${fmtNum(MIN_WP_BOUNDARY_DIST_M)} m (Gefahr nächtlicher Lärmbeschwerden > 35 dB(A) am Nachbarfenster). Schallschutzhaube oder Standortanpassung empfohlen.`,
          });
        } else {
          findings.push({
            status: "ok",
            message: `Außen-Wärmepumpe mit ${fmtNum(wpDist)} m Grenzabstand hält den Schallschutzabstand von mind. ${fmtNum(MIN_WP_BOUNDARY_DIST_M)} m ein (TA Lärm Nachbarschutz plausibel).`,
          });
        }
      } else {
        findings.push({
          status: "warn",
          message: "Außen-Wärmepumpe vorhanden, aber kein Grenzabstand (abstand_waermepumpe_grenze_m) angegeben – TA-Lärm-Prüfung unvollständig.",
        });
      }
    }

    // 2. Lärmpegelbereiche DIN 4109 (LPB IV bis VII erfordern erhöhte Schalldämmung der Fenster)
    if (lpb !== undefined) {
      const lpbNum = Number(lpb);
      if (isNum(lpbNum) && lpbNum >= 4) {
        findings.push({
          status: "warn",
          message: `Erhöhter Lärmpegelbereich ${lpbNum} nach DIN 4109 deklariert – Außenbauteile und Fenster erfordern erhöhten bewerteten Schalldämm-Maß-Nachweis R'w,res (Schallschutzverglasung Klasse 3–4).`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Lärmpegelbereich ${lpb} liegt im Standardbereich der DIN 4109 (Regelanforderungen an Außenwand/Fenster ausreichend).`,
        });
      }
    }

    if (findings.length === 0) {
      findings.push({
        status: "ok",
        message: "Schallschutz Außenbauteile & Haustechnik im Normbereich (keine kritischen Außenaggregate deklariert).",
      });
    }

    return findings;
  },
};

