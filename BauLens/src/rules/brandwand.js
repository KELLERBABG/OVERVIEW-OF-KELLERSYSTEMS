import { fmtNum, isNum } from "../util.js";

export const DEFAULT_BRANDWAND_DIST_M = 2.5;

export default {
  id: "brandwand",
  para: "§30 Abs. 2 MBO",
  title: "Brandwand-Erfordernis bei Grenzbebauung (< 2,50 m zur Nachbargrenze)",
  evaluate(spec, ctx = {}) {
    const grenz = spec?.grundstueck?.grenzabstaende;
    const thresholdM = ctx.params?.thresholdM ?? DEFAULT_BRANDWAND_DIST_M;

    if (typeof grenz !== "object" || grenz === null || Array.isArray(grenz) || Object.keys(grenz).length === 0) {
      return [{
        status: "warn",
        message: "Keine Grenzabstände angegeben – Prüfung des Brandwand-Erfordernisses nicht möglich.",
      }];
    }

    const closeBoundaries = [];
    for (const [seite, dist] of Object.entries(grenz)) {
      if (isNum(dist) && dist < thresholdM) {
        closeBoundaries.push({ seite, dist });
      }
    }

    if (closeBoundaries.length === 0) {
      return [{
        status: "ok",
        message: `Alle Gebäudeaußenwände halten mindestens ${fmtNum(thresholdM)} m Grenzabstand ein – keine gebäudeabschlussseitige Brandwand gem. § 30 Abs. 2 MBO / OIB RL 2 erforderlich.`,
      }];
    }

    const sidesDesc = closeBoundaries.map((b) => `${b.seite} (${fmtNum(b.dist)} m)`).join(", ");
    return [{
      status: "warn",
      message: `Grenzabstand < ${fmtNum(thresholdM)} m an: ${sidesDesc}. Abschlusswand muss zwingend als feuerbeständige Brandwand (F90-A / REI 90-M) ohne ungeschützte Öffnungen ausgeführt werden (§ 30 MBO / OIB RL 2).`,
    }];
  },
};

