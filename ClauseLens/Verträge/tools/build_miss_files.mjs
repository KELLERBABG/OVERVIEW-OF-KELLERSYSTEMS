import { readFileSync, writeFileSync, mkdirSync, readdirSync } from "node:fs";
import { join } from "node:path";
const ROOT = "Verträge";
const s = JSON.parse(readFileSync(join(ROOT, "reports", "scorecard.json"), "utf8"));
const slices = ["marketing", "immobilien", "service", "evasion"];
mkdirSync(join(ROOT, "proposals"), { recursive: true });
const buckets = Object.fromEntries(slices.map((x) => [x, []]));
const unparsed = [];
for (const m of s.misses || []) {
  const mm = String(m).match(/^([a-z_]+) @([a-z0-9-]+):L(\d+)(?:-(\d+))? \[(\w+)\]/);
  if (!mm) { unparsed.push(String(m)); continue; }
  const slice = mm[2].split("-")[0];
  if (!buckets[slice]) continue;
  const [, type, slug, line] = mm;
  const text = readFileSync(join(ROOT, "corpus", slug + ".txt"), "utf8");
  const paras = text.split(/\n\s*\n/);
  const para = paras.find((p) => p.split(/\n/).some((_, i) => false)) || "";
  let hit = "";
  let cursor = 0;
  for (const p of paras) {
    const count = p.split(/\n/).length;
    if (+line >= cursor + 1 && +line < cursor + 1 + count) { hit = p; break; }
    cursor += count + 1;
  }
  let note = "";
  try {
    const L = JSON.parse(readFileSync(join(ROOT, "corpus", "labels", slug + ".json"), "utf8"));
    const trap = (L.traps || []).find((t) => t.expectedType === type && hit.includes(t.marker));
    note = trap ? String(trap.note || "") : "";
  } catch {}
  buckets[slice].push("### " + slug + ":L" + line + "  type=" + type + "\nnote: " + note.replace(/\s+/g, " ").slice(0, 220) + "\n" + hit.replace(/\s+/g, " ").trim() + "\n");
}
for (const slice of slices) {
  const body = "# " + slice + " engine misses (" + buckets[slice].length + ")\nEach block: slug:line, expected type, label technique note, and the full marker paragraph.\n\n" + buckets[slice].join("\n");
  writeFileSync(join(ROOT, "proposals", "misses_" + slice + ".txt"), body);
  console.log(slice + " " + buckets[slice].length + " misses written");
}
console.log("UNPARSED " + unparsed.length);
for (const u of unparsed.slice(0, 15)) console.log("  " + u.slice(0, 200));
