import { fmtNum, isNum } from "../util.js";

export default {
  id: "barrierefreiheit",
  para: "§50 MBO / DIN 18040-2",
  title: "Barrierefreiheit & Aufzugspflicht (DIN 18040-2 / OIB RL 4 / SIA 500)",
  evaluate(spec, ctx = {}) {
    const b = spec?.gebaeude ?? {};
    const geschosse = b.geschosse_ueber_grund;
    const h = b.hoehe_m;
    const we = b.wohnungen_anzahl ?? 1;
    const aufzug = b.aufzug_vorhanden;
    const stufenlos = b.eingang_stufenlos;
    const rampeneigung = b.rampen_neigung_prozent;

    const findings = [];

    // 1. Aufzugspflicht: ab 4 Vollgeschossen oder Höhe > 13 m (§39/50 MBO, OIB RL 4, PBG ZH)
    const requiresElevator = (isNum(geschosse) && geschosse >= 4) || (isNum(h) && h > 13.0);
    if (requiresElevator) {
      if (aufzug === true) {
        findings.push({
          status: "ok",
          message: `Personenaufzug vorhanden – Aufzugspflicht ab ${geschosse ?? '?'} Geschossen / Höhe ${fmtNum(h)} m erfüllt (§ 39/50 MBO, OIB RL 4, SIA 500).`,
        });
      } else if (aufzug === false) {
        findings.push({
          status: "fail",
          message: `Gebäude mit ${geschosse ?? '?'} Vollgeschossen / Höhe ${fmtNum(h)} m erfordert zwingend einen Personenaufzug (§ 50 MBO / Art. 13 OIB RL 4 / SIA 500) – fehlt im Nachweis.`,
        });
      } else {
        findings.push({
          status: "warn",
          message: `Gebäude mit ${geschosse ?? '?'} Vollgeschossen / Höhe ${fmtNum(h)} m unterliegt der gesetzlichen Aufzugspflicht – bitte Aufzugsnachweis im Entwurf deklarieren.`,
        });
      }
    }

    // 2. Stufenlose Erschließung bei Mehrfamilienhäusern (ab 3 WE)
    if (we >= 3) {
      if (stufenlos === true) {
        if (isNum(rampeneigung) && rampeneigung > 6.0) {
          findings.push({
            status: "warn",
            message: `Hauseingang stufenlos, aber Rampenneigung ${fmtNum(rampeneigung)} % übersteigt maximal zulässige 6.0 % gem. DIN 18040-2 / SIA 500.`,
          });
        } else {
          findings.push({
            status: "ok",
            message: `Stufenloser Hauseingang mit barrierefreier Erschließung vorhanden (DIN 18040-2 / SIA 500 konform).`,
          });
        }
      } else if (stufenlos === false) {
        findings.push({
          status: "warn",
          message: `Mehrfamilienhaus (${we} WE) ohne stufenlosen Zugang – Barrierefreiheit gem. § 50 Abs. 1 MBO / DIN 18040-2 nicht nachgewiesen.`,
        });
      }
    }

    if (findings.length === 0) {
      findings.push({
        status: "ok",
        message: "Keine erhöhten Sonderanforderungen an Aufzug oder Großwohnanlage (unterhalb Aufzugsschwelle).",
      });
    }

    return findings;
  },
};

