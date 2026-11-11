import { fmtNum, isNum } from "../util.js";

export const DEFAULT_GFZ_LIMIT = 1.2;

export default {
  id: "gfz",
  para: "§20 BauNVO",
  title: "Geschossflächenzahl (GFZ) gegenüber Bebauungsplan-Grenzwert",
  evaluate(spec) {
    const ist = spec?.gebaeude?.gfz_ist;
    const limit = spec?.einstellungen?.gfz_limit;

    // Falls weder Ist noch Limit angegeben sind, keine Pflichtprüfung (z.B. §34 BauGB ohne B-Plan)
    if (!isNum(ist)) {
      if (isNum(limit)) {
        return [{
          status: "warn",
          message: `GFZ nicht angegeben – Vergleich mit dem konfigurierten B-Plan-Grenzwert ${fmtNum(limit)} nicht möglich.`,
        }];
      }
      return [];
    }

    const effectiveLimit = isNum(limit) ? limit : DEFAULT_GFZ_LIMIT;

    if (ist > effectiveLimit + 1e-9) {
      return [{
        status: "fail",
        message: `GFZ ist ${fmtNum(ist)} überschreitet das zulässige Höchstmaß der baulichen Nutzung von ${fmtNum(effectiveLimit)} (§20 BauNVO). Erfordert zwingend eine Befreiung nach §31 BauGB vor Bauantragseinreichung.`,
      }];
    }

    return [{
      status: "ok",
      message: `GFZ ist ${fmtNum(ist)} hält das zulässige Höchstmaß von ${fmtNum(effectiveLimit)} (§20 BauNVO) ein.`,
    }];
  },
};
