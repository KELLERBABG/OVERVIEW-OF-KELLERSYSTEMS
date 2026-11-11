import { fmtNum, isNum } from "../util.js";
import { checkBuildingWithinBaugrenze } from "../bplanparser.js";

export default {
  id: "bplan_festsetzungen",
  para: "§9 / §30 BauGB, §22 / §23 BauNVO",
  title: "Bebauungsplan-Festsetzungen (Art & Maß, Höhen, Baufenster, Dachform)",
  evaluate(spec, ctx = {}) {
    const bplan = spec?.bplan;
    if (!bplan || typeof bplan !== "object") {
      return [];
    }

    const b = spec?.gebaeude ?? {};
    const findings = [];

    // 1. Firsthöhe (FH)
    const fhMax = bplan.firsthoehe_max_m ?? bplan.max_firsthoehe_m;
    const fhIst = b.firsthoehe_m ?? b.hoehe_m;
    if (isNum(fhMax) && isNum(fhIst)) {
      if (fhIst > fhMax + 1e-4) {
        findings.push({
          status: "fail",
          message: `Firsthöhe ${fmtNum(fhIst)} m überschreitet die B-Plan-Festsetzung von max. ${fmtNum(fhMax)} m (§ 9 Abs. 1 Nr. 1 BauGB, § 16/18 BauNVO) – Befreiung nach § 31 Abs. 2 BauGB erforderlich.`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Firsthöhe ${fmtNum(fhIst)} m hält die zulässige Maximalhöhe von ${fmtNum(fhMax)} m ein.`,
        });
      }
    }

    // 2. Traufhöhe (TH)
    const thMax = bplan.traufhoehe_max_m ?? bplan.max_traufhoehe_m;
    const thIst = b.traufhoehe_m;
    if (isNum(thMax) && isNum(thIst)) {
      if (thIst > thMax + 1e-4) {
        findings.push({
          status: "fail",
          message: `Traufhöhe ${fmtNum(thIst)} m überschreitet die B-Plan-Festsetzung von max. ${fmtNum(thMax)} m (§ 18 BauNVO) – unzulässig.`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Traufhöhe ${fmtNum(thIst)} m hält die zulässige Maximalhöhe von ${fmtNum(thMax)} m ein.`,
        });
      }
    }

    // 3. Zahl der Vollgeschosse (Z)
    const zMax = bplan.vollgeschosse_max;
    const zIst = b.geschosse_ueber_grund;
    if (isNum(zMax) && isNum(zIst)) {
      if (zIst > zMax) {
        findings.push({
          status: "fail",
          message: `Gebäude mit ${zIst} Vollgeschossen überschreitet das im B-Plan festgesetzte Höchstmaß von ${zMax} Vollgeschossen (§ 20 Abs. 1 BauNVO).`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Zahl der Vollgeschosse (${zIst}) hält die Vorgabe von max. ${zMax} ein.`,
        });
      }
    }

    // 4. Dachform & Dachneigung
    const zulDach = bplan.zulaessige_dachformen;
    const dachformIst = b.dachform ?? b.dach?.form;
    if (Array.isArray(zulDach) && zulDach.length > 0 && dachformIst) {
      const match = zulDach.some((d) => d.toLowerCase() === String(dachformIst).toLowerCase());
      if (!match) {
        findings.push({
          status: "fail",
          message: `Dachform "${dachformIst}" weicht von den zulässigen B-Plan-Dachformen ab (${zulDach.join(", ")}) gem. örtlicher Bauvorschrift (§ 9 Abs. 4 BauGB).`,
        });
      } else {
        findings.push({
          status: "ok",
          message: `Dachform "${dachformIst}" entspricht den B-Plan-Festsetzungen.`,
        });
      }
    }

    const dnMin = bplan.dachneigung_min_grad;
    const dnMax = bplan.dachneigung_max_grad;
    const dnIst = b.dachneigung_grad ?? b.dach?.dachneigung_grad;
    if (isNum(dnIst)) {
      if (isNum(dnMin) && dnIst < dnMin) {
        findings.push({
          status: "fail",
          message: `Dachneigung ${fmtNum(dnIst)}° unterschreitet die festgesetzte Mindestneigung von ${fmtNum(dnMin)}°.`,
        });
      } else if (isNum(dnMax) && dnIst > dnMax) {
        findings.push({
          status: "fail",
          message: `Dachneigung ${fmtNum(dnIst)}° überschreitet die festgesetzte Maximalneigung von ${fmtNum(dnMax)}°.`,
        });
      } else if (isNum(dnMin) || isNum(dnMax)) {
        findings.push({
          status: "ok",
          message: `Dachneigung ${fmtNum(dnIst)}° liegt innerhalb des festgesetzten Bereichs (${dnMin ?? 0}° - ${dnMax ?? 90}°).`,
        });
      }
    }

    // 5. Baugrenze & Baufenster (geometrisch)
    if (bplan.baugrenze_polygon && Array.isArray(bplan.baugrenze_polygon) && bplan.baugrenze_polygon.length >= 3) {
      const buildingPoly = b.umriss_polygon ?? spec?.grundstueck?.gebaeude_polygon;
      if (buildingPoly && Array.isArray(buildingPoly) && buildingPoly.length >= 3) {
        const geoCheck = checkBuildingWithinBaugrenze(buildingPoly, bplan.baugrenze_polygon);
        if (!geoCheck.valid) {
          findings.push({
            status: "fail",
            message: `Gebäudeumriss ragt über die festgesetzte Baugrenze (BP_Baugrenze) hinaus (${geoCheck.violations.length} Eckpunkte außerhalb) – Verstoß gegen § 23 Abs. 3 BauNVO.`,
          });
        } else {
          findings.push({
            status: "ok",
            message: `Gebäudekörper liegt vollständig innerhalb des überbaubaren Baufensters (Baugrenze eingehalten gem. § 23 BauNVO).`,
          });
        }
      }
    } else if (bplan.baugrenze_eingehalten === false) {
      findings.push({
        status: "fail",
        message: `Baugrenze laut Projektangabe nicht eingehalten – unzulässiges Vortreten über das Baufenster (§ 23 Abs. 3 BauNVO).`,
      });
    } else if (bplan.baugrenze_eingehalten === true) {
      findings.push({
        status: "ok",
        message: `Baugrenze des B-Plans laut Angabe eingehalten (§ 23 BauNVO).`,
      });
    }

    return findings;
  },
};

