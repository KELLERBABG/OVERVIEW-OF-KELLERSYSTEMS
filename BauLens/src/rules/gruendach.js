import { isNum } from "../util.js";

export default {
  id: "gruendach",
  para: "§89 LBO / Gründachsatzung",
  title: "Dachbegrünung & Freiflächengestaltung",
  evaluate(spec, ctx = {}) {
    const muni = ctx.municipality;
    const gruendachSatzung = muni?.satzungen?.gruendach;

    // Nur prüfen, wenn entweder explizit eine Gründachprüfung verlangt ist oder Dachdaten vorhanden sind
    const dachneigung = spec?.gebaeude?.dachneigung_grad ?? spec?.gebaeude?.dach?.dachneigung_grad;
    const dachform = (spec?.gebaeude?.dachform ?? spec?.gebaeude?.dach?.form ?? "").toLowerCase();
    const gruendachVorhanden = spec?.gebaeude?.gruendach_vorhanden ?? spec?.gebaeude?.dach?.gruendach_vorhanden;
    const substratStaerkeCm = spec?.gebaeude?.gruendach_substrat_cm ?? spec?.gebaeude?.dach?.substrat_cm;

    if (!gruendachSatzung && !spec?.einstellungen?.gruendach_pflicht) {
      return [];
    }

    // Wenn keine Dachangaben vorliegen und keine explizite Pflicht forciert ist: keine störende Warnung generieren
    if (dachneigung === undefined && !dachform && gruendachVorhanden === undefined && !spec?.einstellungen?.gruendach_pflicht) {
      return [];
    }

    const findings = [];
    const para = gruendachSatzung?.para ?? "Örtliche Bauvorschrift gem. LBO";
    const cityName = muni?.name ?? "Kommune";

    // Prüfen, ob es sich um ein Flachdach oder flach geneigtes Dach handelt
    const isFlach = (isNum(dachneigung) && dachneigung <= (gruendachSatzung?.maxGrad ?? 15)) ||
                    dachform.includes("flach") ||
                    dachform.includes("pult");

    if (!isFlach && isNum(dachneigung) && dachneigung > (gruendachSatzung?.maxGrad ?? 15)) {
      // Steildach – Gründachpflicht greift in der Regel nicht
      return [{
        status: "ok",
        message: `Steildach (${dachneigung}°): Befreit von der kommunalen Gründachpflicht nach ${para}.`,
      }];
    }

    if (dachneigung === undefined && !dachform) {
      findings.push({
        status: "warn",
        message: `In ${cityName} gilt die ${gruendachSatzung.title}. Dachneigung und Dachform im Bauantrag spezifizieren zur Überprüfung der Begrünungspflicht.`,
      });
      return findings;
    }

    if (isFlach) {
      if (gruendachVorhanden === false) {
        findings.push({
          status: "fail",
          message: `Verstoß gegen ${gruendachSatzung.title} (${para}): Flachdächer bis ${gruendachSatzung.maxGrad}° müssen im Stadtgebiet ${cityName} extensiv oder intensiv begrünt werden. Begrünungsnachweis fehlt im Entwurf.`,
        });
      } else if (gruendachVorhanden === true) {
        // Substratstärke prüfen falls vorhanden
        const minCm = gruendachSatzung.minSubstratStaerkeCm ?? 10;
        if (isNum(substratStaerkeCm) && substratStaerkeCm < minCm) {
          findings.push({
            status: "warn",
            message: `Gründach deklariert, jedoch Substrataufbau (${substratStaerkeCm} cm) unter kommunalem Mindestmaß von ${minCm} cm (${gruendachSatzung.title}).`,
          });
        } else {
          findings.push({
            status: "ok",
            message: `Kommunale Dachbegrünungspflicht erfüllt gem. ${gruendachSatzung.title} (${para}).`,
          });
        }
      } else {
        // Nicht deklariert
        findings.push({
          status: "warn",
          message: `Flachdach ohne Begrünungsdeklaration in ${cityName}. Gem. ${gruendachSatzung.title} (${para}) ist eine Dachbegrünung im Bauantrag nachzuweisen.`,
        });
      }
    }

    return findings;
  },
};
