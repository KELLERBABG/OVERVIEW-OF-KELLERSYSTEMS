import { isNum } from "../util.js";

export default {
  id: "stellplaetze",
  para: "§48 MBO / LBO",
  title: "Notwendige Kraftfahrzeug- und Fahrradstellplätze",
  evaluate(spec, ctx = {}) {
    const wohnungen = spec?.gebaeude?.wohnungen_anzahl;
    const pkwIst = spec?.gebaeude?.stellplaetze_ist;
    const radIst = spec?.gebaeude?.fahrradstellplaetze_ist;

    const muni = ctx.municipality;
    const satzung = muni?.satzungen?.stellplatz;

    // Nur prüfen, wenn Wohnungen angegeben sind und entweder Stellplätze deklariert oder Nachweispflicht aktiviert
    if (!isNum(wohnungen) || wohnungen <= 0) {
      if (isNum(pkwIst) || isNum(radIst)) {
        return [{
          status: "warn",
          message: "Stellplätze angegeben, aber Wohnungsanzahl fehlt – Verifikation der Soll-Werte nicht möglich.",
        }];
      }
      return [];
    }

    // Wenn keine Stellplätze im Spec hinterlegt sind und keine Faktoren gefordert sind, optional behandeln
    if (!isNum(pkwIst) && !isNum(radIst) && !spec?.einstellungen?.stellplatznachweis_erforderlich && !satzung) {
      return [];
    }

    // Municipal override > spec einstellungen > default 1.0
    const pkwFaktor = satzung?.pkwFaktor !== undefined 
      ? satzung.pkwFaktor 
      : (spec?.einstellungen?.pkw_stellplatz_faktor ?? 1.0);

    const radFaktor = satzung?.radFaktor !== undefined
      ? satzung.radFaktor
      : (spec?.einstellungen?.rad_stellplatz_faktor ?? 2.0);

    const pkwSoll = Math.ceil(wohnungen * pkwFaktor);
    const radSoll = Math.ceil(wohnungen * radFaktor);

    const findings = [];
    const satzungsLabel = satzung ? `gem. ${satzung.title}` : "gem. Landesbauordnung";

    // PKW Prüfung
    if (pkwFaktor === 0.0) {
      // Spezieller Fall z.B. Berlin BauO Bln: Keine Kfz-Stellplatzpflicht für Wohnen
      findings.push({
        status: "ok",
        message: `${satzungsLabel}: Keine Kfz-Stellplatzpflicht für Wohnungen (${pkwIst ?? 0} Pkw deklariert).`,
      });
    } else if (!isNum(pkwIst)) {
      findings.push({
        status: "warn",
        message: `Pkw-Stellplätze nicht deklariert (Soll: mind. ${pkwSoll} Stellplätze bei ${wohnungen} WE ${satzungsLabel}). Nachweispflicht im Bauantrag beachten.`,
      });
    } else if (pkwIst < pkwSoll) {
      findings.push({
        status: "fail",
        message: `Pkw-Stellplatzdefizit: ${pkwIst} nachgewiesen, erforderlich sind mindestens ${pkwSoll} (${pkwFaktor} pro WE). Fehlende Stellplätze müssen abgelöst oder nachgewiesen werden.`,
      });
    } else {
      findings.push({
        status: "ok",
        message: `Pkw-Stellplatznachweis erfüllt (${pkwIst} von mind. ${pkwSoll} vorhanden).`,
      });
    }

    // Fahrrad Prüfung (nur wenn Fahrradplätze deklariert oder Fahrradsatzung vorliegt und radIst angegeben ist)
    if (isNum(radIst)) {
      if (radIst < radSoll) {
        findings.push({
          status: "warn",
          message: `Fahrradabstellplätze unzureichend: ${radIst} vorhanden, empfohlen/gefordert sind ${radSoll} (${radFaktor} pro WE).`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Fahrradstellplatznachweis erfüllt (${radIst} von mind. ${radSoll} vorhanden).`,
        });
      }
    }

    return findings;
  },
};
