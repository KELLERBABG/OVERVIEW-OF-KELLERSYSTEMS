import { fmtNum, isNum } from "../util.js";

export const DEFAULT_PARAMS = { ratio: 1 / 8 };

export default {
  id: "tageslicht",
  para: "§49 MBO",
  title: "Belichtung von Aufenthaltsräumen: Lichtfläche >= 1/8 der Raumgrundfläche",
  evaluate(spec, ctx = {}) {
    const { ratio = DEFAULT_PARAMS.ratio } = ctx.params ?? {};
    const raeume = spec?.gebaeude?.aufenthaltsraeume;
    if (!Array.isArray(raeume) || raeume.length === 0) {
      return [{ status: "warn", message: "Keine Aufenthaltsräume angegeben – Tageslichtprüfung (1/8-Regel) ohne Daten." }];
    }

    const findings = [];
    const unvollstaendig = [];
    let okCount = 0;

    for (const raum of raeume) {
      const name = typeof raum?.name === "string" && raum.name.trim() !== "" ? raum.name : "Raum (ohne Namen)";
      const flaeche = raum?.flaeche_m2;
      const fenster = isNum(raum?.fensterflaeche_m2) ? raum.fensterflaeche_m2 : raum?.fenster_flaeche_m2;
      if (!isNum(flaeche) || !isNum(fenster)) {
        unvollstaendig.push(name);
        continue;
      }
      const erforderlich = flaeche * ratio;
      if (fenster + 1e-9 >= erforderlich) {
        okCount += 1;
      } else {
        findings.push({
          status: "fail",
          message: `${name}: Fensterfläche ${fmtNum(fenster)} m² < 1/8 der Raumgrundfläche (${fmtNum(flaeche)} m²); mindestens ${fmtNum(erforderlich)} m² Lichtöffnungsfläche erforderlich.`,
        });
      }
    }

    if (unvollstaendig.length > 0) {
      findings.push({
        status: "warn",
        message: `Fehlende Flächenangaben bei: ${unvollstaendig.join(", ")} – diese Räume konnten nicht geprüft werden.`,
      });
    }
    if (okCount > 0 && !findings.some((f) => f.status === "fail")) {
      findings.push({
        status: "ok",
        message: `Alle ${okCount} prüfbaren Aufenthaltsräume erfüllen das 1/8-Verhältnis (Fensterfläche zu Raumgrundfläche).`,
      });
    }
    if (findings.length === 0) {
      findings.push({ status: "warn", message: "Kein Aufenthaltsraum mit vollständigen Angaben gefunden." });
    }
    return findings;
  },
};
