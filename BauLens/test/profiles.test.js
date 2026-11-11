import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { execFileSync } from "node:child_process";
import { runChecks } from "../src/engine.js";
import { PROFILES, PROFILE_CODES, getProfile, resolveProfileCode, formatLandsTable } from "../src/profiles.js";
import { getRuleById } from "../src/rules/index.js";

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const bin = join(root, "bin", "baucheck.js");
const fixture = (name) => JSON.parse(readFileSync(join(root, "test", "fixtures", name), "utf8"));

function runCli(args) {
  try {
    const stdout = execFileSync(process.execPath, [bin, ...args], { encoding: "utf8" });
    return { code: 0, stdout };
  } catch (err) {
    return { code: err.status ?? 1, stdout: err.stdout ?? "", stderr: err.stderr ?? "" };
  }
}

test("Registry enthaelt MBO, alle 16 Bundeslaender sowie AT und CH Profile", () => {
  assert.equal(PROFILE_CODES.length, 34);
  for (const code of ["MBO", "BW", "BY", "BE", "BB", "HB", "HH", "HE", "MV", "NI", "NW", "RP", "SL", "SN", "ST", "SH", "TH", "AT_OIB", "AT_W", "CH_IVHB", "CH_ZH"]) {
    assert.ok(getProfile(code), `Profil ${code} fehlt`);
  }
});

test("Jede Regel-Override verweist auf eine bekannte Regel mit gueltigen Feldern", () => {
  for (const [code, profile] of Object.entries(PROFILES)) {
    for (const [ruleId, override] of Object.entries(profile.ruleOverrides)) {
      assert.ok(
        getRuleById(ruleId) !== null,
        `${code}: Override für unbekannte Regel "${ruleId}"`
      );
      assert.ok(
        override.enabled === undefined || typeof override.enabled === "boolean",
        `${code}/${ruleId}: enabled muss boolean oder undefined sein`
      );
      if (override.params !== undefined) {
        assert.equal(typeof override.params, "object");
        for (const v of Object.values(override.params)) assert.ok(v === undefined || typeof v === "number");
      }
      if (override.para !== undefined) assert.equal(typeof override.para, "string");
    }
    for (const extra of profile.extraRules) {
      assert.ok(getRuleById(extra) !== null, `${code}: extraRule "${extra}" unbekannt`);
    }
    assert.ok(Array.isArray(profile.notes));
    assert.equal(typeof profile.citationBase, "string");
  }
});

test("Bayern: abstandsflaeche deaktiviert, grenzbebauung feuert als Warnung", () => {
  const res = runChecks(fixture("bad_spec.json"), { land: "BY" });
  assert.equal(res.profile.code, "BY");
  const ids = new Set(res.results.map((r) => r.id));
  assert.ok(!ids.has("abstandsflaeche"), "abstandsflaeche darf in BY nicht laufen");
  assert.ok(ids.has("grenzbebauung"), "grenzbebauung muss in BY laufen");

  const grenz = res.results.find((r) => r.id === "grenzbebauung");
  assert.ok(grenz.findings.every((f) => f.status === "warn"));
});

test("Bayern: good_spec bleibt ohne Fehler", () => {
  const res = runChecks(fixture("good_spec.json"), { land: "BY" });
  assert.equal(res.counts.fail, 0);
});

test("Setback-Erwartung unterscheidet sich zwischen MBO, BW und BY", () => {
  const spec = {
    grundstueck: { grenzabstaende: { norden: 4.0 } },
    gebaeude: { hoehe_m: 10 },
  };

  const mbo = runChecks(spec, { land: "MBO" }).results.find((r) => r.id === "abstandsflaeche");
  assert.match(mbo.findings[0].message, /5 m/);

  const bwRes = runChecks(spec, { land: "BW" });
  const bw = bwRes.results.find((r) => r.id === "abstandsflaeche");
  assert.equal(bw.para, "§6 LBO BW");
  assert.match(bw.findings[0].message, /5 m/);

  const byIds = new Set(runChecks(spec, { land: "BY" }).results.map((r) => r.id));
  assert.ok(!byIds.has("abstandsflaeche"));

  assert.notEqual(bw.para, mbo.para);
});

test("Setback: NRW fordert 0,4 H (4,0 m bei 10 m Höhe) gegenüber MBO 0,5 H (5,0 m)", () => {
  const spec = {
    grundstueck: { grenzabstaende: { norden: 4.5 } },
    gebaeude: { hoehe_m: 10 },
  };

  const mbo = runChecks(spec, { land: "MBO" }).results.find((r) => r.id === "abstandsflaeche");
  // In MBO ist 4.5 m < 5.0 m -> fail
  assert.equal(mbo.findings[0].status, "fail");

  const nw = runChecks(spec, { land: "NW" }).results.find((r) => r.id === "abstandsflaeche");
  // In NW ist 4.5 m >= 4.0 m (0,4 H) -> ok
  assert.equal(nw.findings[0].status, "ok");
  assert.equal(nw.para, "§6 Abs. 5 BauO NRW");
});

test("Hoehe 12 m: MBO erfordert 6 m, BW erfordert ebenfalls 6 m aber mit anderem Paragraphen", () => {
  const spec = {
    grundstueck: { grenzabstaende: { norden: 3.0 } },
    gebaeude: { hoehe_m: 12 },
  };
  const mbo = runChecks(spec, { land: "MBO" }).results.find((r) => r.id === "abstandsflaeche");
  const bw = runChecks(spec, { land: "BW" }).results.find((r) => r.id === "abstandsflaeche");
  assert.ok(mbo.findings.some((f) => f.status === "fail"));
  assert.ok(bw.findings.some((f) => f.status === "fail"));
  assert.equal(mbo.para, "§6 Abs. 2 MBO");
  assert.equal(bw.para, "§6 LBO BW");
});

test("spec.bundesland wird genutzt, CLI --land hat Vorrang", () => {
  const spec = fixture("good_spec.json");
  spec.bundesland = "BY";

  const viaSpec = runChecks(spec);
  assert.equal(viaSpec.profile.code, "BY");

  const viaCli = runChecks(spec, { land: "BW" });
  assert.equal(viaCli.profile.code, "BW");
});

test("Unbekannter Landescode liefert kein Ergebnis", () => {
  assert.equal(runChecks(fixture("good_spec.json"), { land: "XX" }), null);
});

test("resolveProfileCode: CLI > Spec > Default", () => {
  assert.equal(resolveProfileCode({ bundesland: "BY" }, "NW"), "NW");
  assert.equal(resolveProfileCode({ bundesland: " by " }, null), "BY");
  assert.equal(resolveProfileCode({}, null), "MBO");
});

test("Bericht enthaelt Profilname und LBO-Zitatbasis", () => {
  const res = runChecks(fixture("bad_spec.json"), { land: "BY" });
  assert.equal(res.profile.code, "BY");
  const abstandsklausel = res.results.find((r) => r.id === "gebaeudeklasse");
  assert.match(abstandsklausel.para, /BayBO/);
});

test("CLI --lands gibt Tabelle aus (Exit 0)", () => {
  const out = runCli(["--lands"]);
  assert.equal(out.code, 0);
  assert.match(out.stdout, /Code/);
  assert.match(out.stdout, /Bayern/);
  assert.match(out.stdout, /Grenzbebauung/);
});

test("CLI check mit --land BY auf bad_spec: Exit 1, ohne abstandsflaeche-Fehler", () => {
  const file = join(root, "test", "fixtures", "bad_spec.json");
  const out = runCli(["check", file, "--land", "BY"]);
  assert.equal(out.code, 1);
  assert.match(out.stdout, /Landesprofil: BY/);
  assert.doesNotMatch(out.stdout, /abstandsflaeche \(§6 Abs\. 2 MBO\)/);
  assert.match(out.stdout, /grenzbebauung/);
});

test("CLI check mit unbekanntem --land: Exit 2", () => {
  const file = join(root, "test", "fixtures", "good_spec.json");
  const out = runCli(["check", file, "--land", "XX"]);
  assert.equal(out.code, 2);
  assert.match(out.stderr, /Unbekannter Landescode/);
});

test("CLI check ohne --land nutzt bundesland aus dem Spec (Exit-Code-Pfad)", async () => {
  const { writeFileSync, unlinkSync } = await import("node:fs");
  const { tmpdir } = await import("node:os");
  const tmpSpec = join(tmpdir(), "baucheck_by_spec.test.json");
  const spec = { ...fixture("good_spec.json"), bundesland: "BE" };
  try {
    writeFileSync(tmpSpec, JSON.stringify(spec), "utf8");
    const out = runCli(["check", tmpSpec]);
    assert.equal(out.code, 0);
    assert.match(out.stdout, /Landesprofil: BE/);
  } finally {
    try { unlinkSync(tmpSpec); } catch { /* ignore */ }
  }
});

test("formatLandsTable enthaelt alle Codes und Ausrichtung", () => {
  const table = formatLandsTable();
  for (const code of PROFILE_CODES) assert.ok(table.includes(code));
});
