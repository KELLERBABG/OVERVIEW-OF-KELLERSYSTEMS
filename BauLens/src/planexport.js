/**
 * BauLens - naming the Lageplan files of a workspace export.
 *
 * A workspace export writes one Blatt per Vorhaben into one folder. That folder
 * is the submission folder that goes to the Bauamt, so two properties matter
 * more than elegance:
 *
 *   1. **No name may leave the chosen folder.** The label comes from a project
 *      directory name, i.e. from a path on somebody's disk. A name with a
 *      separator or a drive letter in it would write somewhere else entirely, and
 *      `..` would climb out of the folder that was picked in the dialog.
 *   2. **Two Vorhaben must never share one file.** Windows compares file names
 *      case-insensitively, so `Haus` and `haus` are the same file there. If a
 *      collision were resolved by silently overwriting, the folder would hold one
 *      drawing fewer than the report claimed - and the missing one is the plan
 *      somebody would have submitted. Collisions are therefore renamed in a
 *      deterministic way and reported back to the caller.
 *
 * The naming is pure arithmetic over the input, so the same workspace always
 * produces the same file names.
 */

/** The formats a Blatt can be written in; the order is the order they are written. */
export const PLAN_FORMATS = ["dxf", "svg"];

/** Extension per format. Deliberately not derived from the module names. */
const EXTENSIONS = { dxf: "dxf", svg: "svg" };

/** What a label falls back to when nothing usable is left of it. */
const FALLBACK_LABEL = "Lageplan";

/**
 * Makes one path component out of an arbitrary label.
 *
 * Characters that mean something to a filesystem (separators, drive colons, the
 * Windows-reserved set) become a hyphen rather than being dropped, so two labels
 * that differ only in a separator do not silently become the same name. Leading
 * dots and trailing dots/spaces go: a leading dot would make the file hidden,
 * and Windows silently drops trailing dots and spaces, which would turn two
 * distinct names into one.
 */
export function safeLabel(value) {
  /* Whitespace becomes a hyphen rather than being dropped, because dropping it
     would merge two distinct names ("Haus 3" and "Haus3"). */
  const mapped = String(value ?? "")
    .split("")
    .map((character) => {
      const code = character.codePointAt(0);
      if (code < 0x20 || code === 0x7f) return "-";
      if (/\s/.test(character)) return "-";
      return /[\\/:*?"<>|]/.test(character) ? "-" : character;
    })
    .join("");

  /* Order matters: runs collapse first, then the edges are stripped of dots as
     well as hyphens and spaces. Stripping only dots would leave the hyphen that
     a separator became behind - and "..-etc-passwd" still starts with the two
     dots that make a path component path-climbing if a reader ever splits it
     again. */
  const trimmed = mapped
    .replace(/-{2,}/g, "-")
    .replace(/^[.\s-]+/, "")
    .replace(/[.\s-]+$/, "")
    .slice(0, 80)
    .replace(/[.\s-]+$/, "");

  return trimmed === "" || trimmed === "." || trimmed === ".." ? FALLBACK_LABEL : trimmed;
}

/** File name without its directory and extension - the label a plan brings along. */
export function stemOf(filePath) {
  const name = String(filePath ?? "").split(/[\\/]/).pop() ?? "";
  return name.replace(/\.[^.]+$/, "");
}

/**
 * Output file names for a batch, one set per entry and format.
 *
 * `entries` are `{ source, name? }`; without a `name` the plan's own file name is
 * the label. Labels are deduplicated case-insensitively across the *whole* batch,
 * so a suffix is consistent in every format ("Haus-2_Lageplan.dxf" and
 * "Haus-2_Lageplan.svg" belong to the same Vorhaben).
 *
 * Returns `{ names, collisions }`: `names` parallels the entries
 * (`{ label, files: { dxf, svg } }`), and `collisions` names every entry whose
 * label had to be changed, with the reason - the report says "this Blatt is
 * called something else than the folder it came from", which is exactly the
 * information a person needs when the output folder is not what they expected.
 */
export function planOutputNames(entries, formats = PLAN_FORMATS) {
  const taken = new Set();
  const names = [];
  const collisions = [];

  for (const entry of entries) {
    const base = safeLabel(entry.name ?? stemOf(entry.source));
    let label = base;
    let suffix = 2;
    while (taken.has(label.toLowerCase())) {
      label = `${base}-${suffix}`;
      suffix += 1;
    }
    if (label !== base) {
      collisions.push({ source: entry.source, label: base, renamed: label });
    }
    taken.add(label.toLowerCase());

    const files = {};
    for (const format of formats) files[format] = `${label}_Lageplan.${EXTENSIONS[format]}`;
    names.push({ label, files });
  }

  return { names, collisions };
}
