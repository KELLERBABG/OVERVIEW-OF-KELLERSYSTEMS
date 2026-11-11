import { fmtNum, isNum } from "../util.js";

export const DEFAULT_PARAMS = { factor: 0.5, minM: 3 };

const SEITEN_LABEL = {
  norden: "Nordseite",
  nordosten: "Nordostseite",
  osten: "Ostseite",
  suedosten: "Südostseite",
  sueden: "Südseite",
  suedwesten: "Südwestseite",
  westen: "Westseite",
  nordwesten: "Nordwestseite",
};

export default {
  id: "abstandsflaeche",
  para: "§6 Abs. 2 MBO",
  title: "Abstandsflächen zu den Grundstücksgrenzen (H/2, mind. 3 m)",
  evaluate(spec, ctx = {}) {
    const { factor = DEFAULT_PARAMS.factor, minM: MIN_ABSTAND_M = DEFAULT_PARAMS.minM } =
      ctx.params ?? {};
    const h = spec?.gebaeude?.hoehe_m;
    const grenz = spec?.grundstueck?.grenzabstaende;

    if (!isNum(h) || h <= 0) {
      return [{
        status: "warn",
        message: `Gebäudehöhe fehlt – erforderliche Abstandsfläche (H×${fmtNum(factor)}, mind. ${fmtNum(MIN_ABSTAND_M)} m) nicht bestimmbar.`,
      }];
    }
    const erforderlich = Math.max(MIN_ABSTAND_M, h * factor);

    if (typeof grenz !== "object" || grenz === null || Array.isArray(grenz) || Object.keys(grenz).length === 0) {
      return [{
        status: "warn",
        message: `Keine Grenzabstände angegeben – Vergleich mit der erforderlichen Abstandsfläche von ${fmtNum(erforderlich)} m nicht möglich.`,
      }];
    }

    const findings = [];
    let geprueft = 0;
    const okSeiten = [];
    const garage = spec?.gebaeude?.grenzgarage;
    const garageSeite = typeof garage?.grenzseite === "string" ? garage.grenzseite.toLowerCase().trim() : null;

    for (const [seite, ist] of Object.entries(grenz)) {
      const label = SEITEN_LABEL[seite] ?? seite;
      if (!isNum(ist)) continue;
      geprueft += 1;

      // Grenzgaragen- und Nebengebäudeprivileg (§ 6 Abs. 8 MBO / Art. 6 Abs. 7 BayBO / § 6 Abs. 8 BauO NRW)
      const isGarageSide = garage?.vorhanden && (garageSeite === seite || !garageSeite);
      if (isGarageSide && ist < erforderlich) {
        const wh = garage?.wandhoehe_m ?? 3.0;
        const lEinzel = garage?.laenge_einzelgrenze_m ?? 9.0;
        const lGesamt = garage?.laenge_gesamt_m ?? 9.0;

        if (wh <= 3.0 + 1e-9 && lEinzel <= 9.0 + 1e-9 && lGesamt <= 15.0 + 1e-9) {
          findings.push({
            status: "ok",
            message: `${label}: Grenzbebauung/Abstand ${fmtNum(ist)} m ist als privilegierte Grenzgarage/Nebengebäude freigestellt (Wandhöhe ${fmtNum(wh)} m <= 3,0 m, Grenzlänge ${fmtNum(lEinzel)} m <= 9,0 m, Gesamtlänge ${fmtNum(lGesamt)} m <= 15,0 m gem. §6 Abs. 8 MBO/LBO).`,
          });
          continue;
        } else {
          findings.push({
            status: "fail",
            message: `${label}: Grenzgarage überschreitet Privilegierungsgrenzen (Wandhöhe ${fmtNum(wh)} m > 3 m oder Grenzlänge ${fmtNum(lEinzel)} m > 9 m oder Gesamtlänge ${fmtNum(lGesamt)} m > 15 m) – volle Abstandsfläche ${fmtNum(erforderlich)} m erforderlich.`,
          });
          continue;
        }
      }

      if (ist + 1e-9 < erforderlich) {
        findings.push({
          status: "fail",
          message: `${label}: Abstand ${fmtNum(ist)} m < erforderliche Abstandsfläche ${fmtNum(erforderlich)} m (H×${fmtNum(factor)} aus Höhe ${fmtNum(h)} m, mindestens ${fmtNum(MIN_ABSTAND_M)} m).`,
        });
      } else {
        okSeiten.push(`${label} ${fmtNum(ist)} m`);
      }
    }
    if (geprueft === 0) {
      findings.push({ status: "warn", message: "Grenzabstaende enthalten keine verwertbaren Zahlenwerte." });
    } else if (findings.length === 0) {
      findings.push({
        status: "ok",
        message: `Alle ${geprueft} angegebenen Grenzabstände (${okSeiten.join(", ")}) >= erforderliche Abstandsfläche ${fmtNum(erforderlich)} m.`,
      });
    }
    return findings;
  },
};
