import { berechneGebaeudeklasse } from "./gebaeudeklasse.js";

export default {
  id: "rettungswege",
  para: "§11 MBO",
  title: "Rettungswege: zweiter Rettungsweg / Treppenraum ab Gebäudeklasse 4 (vereinfacht)",
  evaluate(spec) {
    const klasse = berechneGebaeudeklasse(spec);
    const gug = spec?.gebaeude?.geschosse_ueber_grund;
    const kritisch = (klasse !== null && klasse >= 4) || (Number.isFinite(gug) && gug >= 5);

    if (!kritisch) {
      return [{
        status: "ok",
        message: `Gebäudeklasse ${klasse ?? "?"} mit höchstens drei Vollgeschossen – Rettung über die notwendige Treppe bzw. vereinfachter Nachweis ausreichend (vereinfachte Prüfung).`,
      }];
    }

    const treppenraum = spec?.gebaeude?.treppenraum_vorhanden === true;
    const zweiterRettungsweg = spec?.gebaeude?.zweiter_rettungsweg_vorhanden === true;

    if (treppenraum && zweiterRettungsweg) {
      return [{
        status: "ok",
        message: "Treppenraum als notwendige Treppe und baulicher zweiter Rettungsweg angegeben.",
      }];
    }
    if (!treppenraum) {
      return [{
        status: "fail",
        message: `Ab Gebäudeklasse 4 muss die notwendige Treppe ein Treppenraum sein und ein baulicher zweiter Rettungsweg vorgehalten werden (§11, §35 Abs. 4 MBO) – Treppenraum${zweiterRettungsweg ? "" : " UND zweiter Rettungsweg"} nicht nachgewiesen.`,
      }];
    }
    return [{
      status: "warn",
      message: "Treppenraum vorhanden, aber kein baulicher zweiter Rettungsweg angegeben – Nachweis vor Antrag führen.",
    }];
  },
};
