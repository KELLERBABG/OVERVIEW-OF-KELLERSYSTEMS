import { fmtNum, isNum } from "../util.js";

export const DEFAULT_PARAMS = { minHoehe: 2.5, reduzierteMindesthoehe: 2.2 };

export default {
  id: "raumhoehen",
  para: "§46 MBO",
  title: "Raumhöhen von Aufenthaltsräumen (mind. 2,50 m mittlere Höhe)",
  evaluate(spec, ctx = {}) {
    const {
      minHoehe: MIN_HOEHE = DEFAULT_PARAMS.minHoehe,
      reduzierteMindesthoehe: REDUZIERTE_MINDESTHOEHE = DEFAULT_PARAMS.reduzierteMindesthoehe,
    } = ctx.params ?? {};
    const raeume = spec?.gebaeude?.aufenthaltsraeume;
    if (!Array.isArray(raeume) || raeume.length === 0) {
      return [{ status: "warn", message: "Keine Aufenthaltsräume angegeben – Raumhöhenprüfung ohne Daten." }];
    }

    const findings = [];
    const unvollstaendig = [];
    let okCount = 0;

    for (const raum of raeume) {
      const name = typeof raum?.name === "string" && raum.name.trim() !== "" ? raum.name : "Raum (ohne Namen)";
      const hoehe = isNum(raum?.raumhoehe_m) ? raum.raumhoehe_m : raum?.lichte_hoehe_m;
      if (!isNum(hoehe)) {
        unvollstaendig.push(name);
        continue;
      }
      if (hoehe + 1e-9 >= MIN_HOEHE) {
        okCount += 1;
      } else if (hoehe + 1e-9 >= REDUZIERTE_MINDESTHOEHE) {
        findings.push({
          status: "warn",
          message: `${name}: mittlere Raumhöhe ${fmtNum(hoehe)} m liegt zwischen ${fmtNum(REDUZIERTE_MINDESTHOEHE)} m und ${fmtNum(MIN_HOEHE)} m – Reduzierung nur im Rahmen der landesrechtlichen Ausnahmen zulässig.`,
        });
      } else {
        findings.push({
          status: "fail",
          message: `${name}: mittlere Raumhöhe ${fmtNum(hoehe)} m unterschreitet die absolute Untergrenze von ${fmtNum(REDUZIERTE_MINDESTHOEHE)} m.`,
        });
      }
    }

    if (unvollstaendig.length > 0) {
      findings.push({
        status: "warn",
        message: `Keine Raumhöhenangabe bei: ${unvollstaendig.join(", ")}.`,
      });
    }
    if (okCount > 0 && !findings.some((f) => f.status === "fail")) {
      findings.push({
        status: "ok",
        message: `Alle ${okCount} prüfbaren Aufenthaltsräume erreichen mindestens ${fmtNum(MIN_HOEHE)} m mittlere Höhe.`,
      });
    }
    return findings;
  },
};
