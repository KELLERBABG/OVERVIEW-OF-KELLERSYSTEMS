import { fmtNum, isNum } from "../util.js";

export const DEFAULT_GRZ_LIMIT = 0.4;

export default {
  id: "grz",
  para: "§19 BauNVO",
  title: "Grundflächenzahl (GRZ) gegenüber Richtwert (Standard 0,4)",
  evaluate(spec) {
    const limit = spec?.einstellungen?.grz_limit ?? DEFAULT_GRZ_LIMIT;
    const ist = spec?.gebaeude?.grz_ist;

    if (!isNum(ist)) {
      return [{
        status: "warn",
        message: `GRZ (überbaute Grundfläche / Grundstücksfläche) nicht angegeben – Vergleich mit dem Richtwert ${fmtNum(limit)} nicht möglich.`,
      }];
    }

    if (ist > limit + 1e-9) {
      return [{
        status: "warn",
        message: `GRZ ist ${fmtNum(ist)} überschreitet den Richtwert ${fmtNum(limit)} (konfigurierbar über einstellungen.grz_limit). Abweichung nur mit Ausnahme/Befreiung zulässig – vor Antrag klären.`,
      }];
    }

    return [{
      status: "ok",
      message: `GRZ ist ${fmtNum(ist)} liegt innerhalb des Richtwerts von ${fmtNum(limit)}.`,
    }];
  },
};
