"use strict";

import { test } from "node:test";
import assert from "node:assert/strict";
import { segmentSentences, extractText, detectFormat } from "../src/textio.js";

test("Abkuerzungen erzeugen keine Satzgrenze", () => {
  const text = "Die Frist betraegt z.B. drei Monate. Sie beginnt am 1.3. zu laufen.";
  const s = segmentSentences(text);
  const texts = s.map((x) => x.text);
  assert.equal(
    texts.length,
    2,
    `erwartet 2 Saetze, erhalten ${texts.length}: ${JSON.stringify(texts)}`
  );
  assert.match(texts[0], /z\.B\. drei Monate\./);
});

test("Nr., Abs. und vgl. werden nicht gesplittet", () => {
  const text = "Nach Nr. 3 gilt Abs. 1. Der Vertrag endet.";
  const texts = segmentSentences(text).map((x) => x.text);
  assert.equal(texts.length, 2);
  assert.match(texts[0], /Abs\. 1\./);
});

test("Klausel-Ueberschrift ohne Punkt startet neuen Satz", () => {
  const text =
    "§4 Haftung\nDie Haftung ist ausgeschlossen.\n\n§5 Laufzeit\nDer Vertrag laeuft ein Jahr.";
  const s = segmentSentences(text);
  const texts = s.map((x) => x.text);
  assert.ok(
    texts.some((t) => t.startsWith("§5")),
    `§5 sollte eigener Satz sein: ${JSON.stringify(texts)}`
  );
  assert.ok(
    !texts.some((t) => /^Die Haftung/.test(t) && t.includes("Laufzeit")),
    "Überschrift darf nicht mit Folgesatz verschmelzen"
  );
});

test("Nummerierte Klauseln (1. und (1)) starten neue Saetze", () => {
  const text = "1. Der Vertrag beginnt am Montag\n(1) Die Laufzeit betraegt ein Jahr.\n(2) Kündigung ist möglich.";
  const texts = segmentSentences(text).map((x) => x.text);
  assert.ok(texts.some((t) => t.startsWith("1.")));
  assert.ok(texts.some((t) => t.startsWith("(1)")));
  assert.ok(texts.some((t) => t.startsWith("(2)")));
});

test("Zahlen ohne Punkt am Zeilenanfang sind KEINE Klauselgrenze", () => {
  const text = "Die Frist betraegt\n12 Monate ab Beginn.";
  const texts = segmentSentences(text).map((x) => x.text);
  assert.equal(texts.length, 1, `nur ein Satz erwartet: ${JSON.stringify(texts)}`);
});

test("Offsets bleiben stabil nach Segmentierung", () => {
  const text = "Erster Satz. Zweiter Satz hier.";
  for (const s of segmentSentences(text)) {
    assert.equal(text.slice(s.start, s.end), s.text);
  }
});

test("Juristische Zitate (S., lit., Art., i.V.m.) werden nicht fälschlich gesplittet", () => {
  const text = "Dies gilt gem. § 307 Abs. 1 S. 2 BGB i. V. m. Art. 6 Abs. 1 lit. f DSGVO vollumfänglich. Eine Abweichung ist unzulässig.";
  const texts = segmentSentences(text).map((x) => x.text);
  assert.equal(texts.length, 2, `erwartet 2 Sätze, erhalten ${texts.length}: ${JSON.stringify(texts)}`);
  assert.match(texts[0], /lit\. f DSGVO/);
});

