import { fmtNum } from "../util.js";

export function berechneGebaeudeklasse(spec) {
  const h = spec?.gebaeude?.hoehe_m;
  const gug = spec?.gebaeude?.geschosse_ueber_grund;
  const units = spec?.gebaeude?.wohnungen_anzahl;
  if (!Number.isFinite(h) || h <= 0) return null;
  if (h > 22) return 5;
  if (h > 13 || gug >= 5) return 4;
  if (h > 11 || units >= 3 || gug >= 4) return 3;
  if (h > 7 || units === 2) return 2;
  return 1;
}

const BESCHREIBUNG = {
  1: "freistehendes Gebäude mit höchstens einer Nutzungseinheit",
  2: "Gebäude bis 11 m Höhe mit wenigen Nutzungseinheiten",
  3: "Gebäude bis 13 m Höhe bzw. bis zu drei Vollgeschossen",
  4: "Gebäude, deren oberster Geschossfußboden höher als 13 m liegt",
  5: "Hochhaus (Höhe über 22 m)",
};

export default {
  id: "gebaeudeklasse",
  para: "§2 MBO",
  title: "Einstufung in eine Gebäudeklasse (1–5)",
  evaluate(spec) {
    const klasse = berechneGebaeudeklasse(spec);
    if (klasse === null) {
      return [{
        status: "warn",
        message: "Keine Gebäudehöhe angegeben – Gebäudeklasse kann nicht bestimmt werden. Folgeregeln arbeiten eingeschränkt.",
      }];
    }
    const b = spec.gebaeude ?? {};
    const teile = [
      `Höhe ${fmtNum(b.hoehe_m)} m`,
      `${b.wohnungen_anzahl ?? "?"} Wohnung(en)`,
      `${b.geschosse_ueber_grund ?? "?"} Vollgeschosse`,
    ];
    return [{
      status: "ok",
      message: `Einstufung: Gebäudeklasse ${klasse} (${BESCHREIBUNG[klasse]}) – ${teile.join(", ")}.`,
    }];
  },
};
