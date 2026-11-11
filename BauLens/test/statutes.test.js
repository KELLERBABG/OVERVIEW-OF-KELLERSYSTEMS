import { test } from "node:test";
import assert from "node:assert/strict";
import { getStatuteByQuery, STATUTES_DB } from "../src/statutes.js";

test("getStatuteByQuery returns correct § 19 BauNVO for GRZ", () => {
  const st = getStatuteByQuery("§19 BauNVO");
  assert.ok(st);
  assert.equal(st.id, "baunvo_19");
  assert.ok(st.wortlaut.includes("50 vom Hundert"));
  assert.ok(st.remedy.includes("§ 31 Abs. 2 BauGB"));
});

test("getStatuteByQuery resolves state-specific setback for BW", () => {
  const st = getStatuteByQuery("§6 LBO BW", "BW");
  assert.ok(st);
  assert.equal(st.id, "lbo_bw_6");
  assert.ok(st.wortlaut.includes("5 m"));
});

test("getStatuteByQuery resolves BayBO boundary regulation for BY", () => {
  const st = getStatuteByQuery("grenzbebauung", "BY");
  assert.ok(st);
  assert.equal(st.id, "baybo_22");
  assert.ok(st.jurisdiction.includes("Bayern"));
});

test("getStatuteByQuery resolves municipal statutes for Munich and Berlin", () => {
  const stMch = getStatuteByQuery("FGS München", "BY", "MUENCHEN");
  assert.ok(stMch);
  assert.equal(stMch.id, "muenchen_gruendach");

  const stBln = getStatuteByQuery("stellplatz", "BE", "BERLIN");
  assert.ok(stBln);
  assert.equal(stBln.id, "berlin_stellplatz");
  assert.ok(stBln.wortlaut.includes("nicht mehr nachweisen"));
});

test("getStatuteByQuery resolves Austrian OIB and Wiener Bauordnung statutes", () => {
  const stOib = getStatuteByQuery("gebaeudeklasse", "AT_OIB");
  assert.ok(stOib);
  assert.equal(stOib.id, "oib_rl_2");
  assert.ok(stOib.wortlaut.includes("Tabelle 1"));

  const stWien = getStatuteByQuery("bauwich", "AT_W");
  assert.ok(stWien);
  assert.equal(stWien.id, "wien_bo_79");
  assert.ok(stWien.norm.includes("BO für Wien"));
  assert.ok(stWien.jurisdiction.includes("Bauordnung für Wien"));
});

test("getStatuteByQuery resolves Swiss IVHB and Zürcher PBG statutes", () => {
  const stIvhb = getStatuteByQuery("abstandsflaeche", "CH_IVHB");
  assert.ok(stIvhb);
  assert.equal(stIvhb.id, "ivhb_grenzabstand");
  assert.ok(stIvhb.wortlaut.includes("Kleiner Grenzabstand"));

  const stZh = getStatuteByQuery("raumhoehe", "CH_ZH");
  assert.ok(stZh);
  assert.equal(stZh.id, "pbg_zh_293");
  assert.ok(stZh.wortlaut.includes("2,40 m"));
});

