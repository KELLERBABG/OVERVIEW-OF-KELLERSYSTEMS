import { isNum } from "../util.js";

export default {
  id: "grenzbebauung",
  para: "§22 BayBO / §6 ff. BauO Bln",
  title: "Grenzbebauung (Bauen an der Grundstücksgrenze)",
  evaluate(spec, ctx = {}) {
    const landCode = ctx.land ?? "MBO";
    const grenz = spec?.grundstueck?.grenzabstaende;
    const findings = [];

    const hatAbstände =
      typeof grenz === "object" && grenz !== null && !Array.isArray(grenz) &&
      Object.values(grenz).some((v) => isNum(v));

    if (hatAbstände) {
      findings.push({
        status: "warn",
        message:
          `Grenzabstände angegeben – im Profil (${landCode}) regelt jedoch das Grenzbebauungsrecht, ob und unter welchen Voraussetzungen an die Grenze gebaut werden darf (Grenzwand, Bebauungsplan bzw. Innenbereich). Bitte planrechtlich bestätigen lassen.`,
      });
    } else {
      findings.push({
        status: "warn",
        message:
          `Keine Grenzabstände angegeben. In ${landCode} entscheidet das Grenzbebauungsregime über das Anreifen an die Nachbargrenze; ohne Angabe ist keine Aussage möglich – Planrecht (Bebauungsplan/§34 BauGB) prüfen.`,
      });
    }

    findings.push({
      status: "warn",
      message:
        "Hinweis: Die Grenzbebauung wird hier nur informativ dargestellt; eine rechtssichere Prüfung erfordert den aktuellen Gesetzestext und den Planfall.",
    });

    return findings;
  },
};
