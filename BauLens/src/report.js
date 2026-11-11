export const DISCLAIMER =
  "Vorprüfung, keine Rechtsberatung; Landesbauordnungen und örtliche Satzungen weichen ab.";

const STATUS_LABEL = { ok: "OK", warn: "WARNUNG", fail: "FEHLER" };
const STATUS_RANK = { fail: 3, warn: 2, ok: 1, none: 0 };

function aggregateStatus(findings) {
  let worst = "none";
  for (const f of findings) {
    if ((STATUS_RANK[f.status] ?? 0) > STATUS_RANK[worst]) worst = f.status;
  }
  return worst;
}

function statusLabel(result) {
  const agg = aggregateStatus(result.findings);
  return agg === "none" ? "-" : STATUS_LABEL[agg];
}

function escapeCell(text) {
  return String(text).replace(/\|/g, "\\|").replace(/\r?\n/g, " ");
}

export function generateMarkdownReport(runResult, meta = {}) {
  const { verdict, counts, results, profile, municipality } = runResult;
  const projekt = typeof meta.projekt === "string" && meta.projekt.trim() !== ""
    ? meta.projekt
    : "(ohne Projektnamen)";
  const datum = meta.datum ?? new Date().toISOString().slice(0, 10);
  const profilCode = profile?.code ?? "MBO";
  const profilName = profile ? `${profile.name} (${profile.citationBase})` : "MBO (Referenz)";

  const lines = [];
  lines.push("# BauLens – Vorprüfungsbericht");
  lines.push("");
  lines.push(`> **Haftungsausschluss:** ${DISCLAIMER}`);
  lines.push("");
  lines.push(`- **Projekt:** ${escapeCell(projekt)}`);
  lines.push(`- **Datum:** ${datum}`);
  lines.push(`- **Landesprofil:** ${profilCode} – ${escapeCell(profilName)}`);
  if (municipality) {
    lines.push(`- **Kommune / Stadt:** ${escapeCell(municipality.name)} (Örtliche Satzungen aktiv)`);
  }
  lines.push(`- **Ergebnis:** **${verdict}** (${counts.fail} Fehler, ${counts.warn} Warnungen, ${counts.ok} OK)`);
  
  if (profile && profile.notes.length > 0) {
    lines.push("");
    lines.push("### Hinweise zum Landesprofil");
    lines.push("");
    for (const note of profile.notes) lines.push(`- ${note}`);
  }

  if (municipality) {
    lines.push("");
    lines.push("### Kommunale Satzungsüberlagerung");
    lines.push("");
    if (municipality.satzungen.stellplatz?.notes) {
      lines.push(`- **Stellplätze:** ${municipality.satzungen.stellplatz.notes}`);
    }
    if (municipality.satzungen.gruendach?.giltFuer) {
      lines.push(`- **Dachbegrünung:** ${municipality.satzungen.gruendach.giltFuer}`);
    }
    if (municipality.satzungen.milieuschutz?.aktiv) {
      lines.push(`- **Erhaltungssatzung (§172 BauGB):** Aktive Milieuschutzgebiete in ${municipality.name}.`);
    }
  }

  lines.push("");
  lines.push("## Zusammenfassung der Regeln");
  lines.push("");
  lines.push("| Regel | Paragraph | Status | Anzahl Befunde |");
  lines.push("|---|---|---|---|");
  for (const r of results) {
    lines.push(
      `| ${escapeCell(r.title)} | ${escapeCell(r.para)} | ${statusLabel(r)} | ${r.findings.length} |`
    );
  }
  lines.push("");

  const fehler = collectByStatus(results, "fail");
  const warnungen = collectByStatus(results, "warn");

  lines.push("## Fehler (vor Antrag beheben)");
  lines.push("");
  if (fehler.length === 0) {
    lines.push("- Keine.");
  } else {
    for (const f of fehler) lines.push(`- [${f.id}] ${f.message}`);
  }
  lines.push("");
  lines.push("## Warnungen (klären / dokumentieren)");
  lines.push("");
  if (warnungen.length === 0) {
    lines.push("- Keine.");
  } else {
    for (const w of warnungen) lines.push(`- [${w.id}] ${w.message}`);
  }
  lines.push("");

  lines.push("## Details je Regel");
  lines.push("");
  for (const r of results) {
    lines.push(`### ${r.para} – ${r.title}`);
    lines.push("");
    if (r.findings.length === 0) {
      lines.push("- Keine Befunde (Eingangsdaten unvollständig oder Regel nicht anwendbar).");
    } else {
      for (const f of r.findings) {
        lines.push(`- **${STATUS_LABEL[f.status] ?? f.status}:** ${f.message}`);
      }
    }
    lines.push("");
  }

  lines.push("---");
  lines.push("");
  lines.push(`**${DISCLAIMER}**`);
  lines.push("");
  const grundlage = profilCode === "MBO"
    ? "MBO 2002 (Fassung 2016) und BauNVO"
    : `Landesprofil ${profilCode}: ${profile?.citationBase ?? "LBO"}, Defaults sonst MBO`;
  lines.push(`Grundlage: ${grundlage}, vereinfachte Formeln und Annahmen siehe docs/RULES.md. Landesprofile und kommunale Satzungen bilden nur bekannte Vorgaben ab – verbindlich ist ausschließlich der jeweils gültige Gesetzestext im Baugenehmigungsverfahren.`);
  lines.push("");

  return lines.join("\n");
}

function collectByStatus(results, status) {
  const out = [];
  for (const r of results) {
    for (const f of r.findings) {
      if (f.status === status) out.push({ id: r.id, para: r.para, message: f.message });
    }
  }
  return out;
}
