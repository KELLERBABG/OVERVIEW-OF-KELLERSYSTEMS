/**
 * Zero-dependency single-file bundler.
 *
 *   node scripts/bundle.js
 *
 * Produces dist/baulens.cjs: every module reachable from bin/baucheck.js in one
 * CommonJS file that runs with plain `node`, and is the input for the
 * single-executable builds (see .github/workflows/release.yml).
 *
 * WHY MODULES ARE SCOPED, NOT CONCATENATED
 * ----------------------------------------
 * Flattening the tree into one scope looks simpler and is wrong: several rule
 * modules declare the same top-level names independently (`DEFAULT_PARAMS` in
 * three of them, `MAX_MITTLERE_GESCHOSSHOEHE`, `isNum`-style helpers). That is
 * legal in separate ES modules and a redeclaration error in one scope. So each
 * module becomes its own function scope, and its public surface is re-exported
 * through an explicit namespace object.
 *
 * The codebase guarantees this stays tractable:
 *   1. no external imports, only `node:` builtins,
 *   2. no top-level side effects in any module,
 *   3. `export default` only ever introduces the name `Rule`,
 *   4. no `export ... from` re-exports and no destructuring exports.
 * Each of those is asserted below rather than assumed.
 */
import { readFileSync, writeFileSync, mkdirSync, existsSync, statSync } from "node:fs";
import { dirname, join, normalize, sep } from "node:path";

const ENTRY = "bin/baucheck.js";
const OUT_DIR = "dist";
const OUT_FILE = join(OUT_DIR, "baulens.cjs");

const pkg = JSON.parse(readFileSync("package.json", "utf8"));
const VERSION = pkg.version;

const posix = (p) => p.split(sep).join("/");
const fail = (msg) => { console.error(`FAILED: ${msg}`); process.exit(1); };

/* --------------------------------------------------------------------------
 * 1. Module graph, dependencies before dependents
 * ------------------------------------------------------------------------ */

function importsOf(file) {
  const code = readFileSync(file, "utf8").replace(/^\uFEFF/, "");
  const out = [];
  const re = /^[ \t]*import\s+([\s\S]*?)\s+from\s+["']([^"']+)["'];?[ \t]*$/gm;
  let m;
  while ((m = re.exec(code))) out.push({ clause: m[1].trim(), spec: m[2] });
  return out;
}

function resolveLocal(from, spec) {
  const base = normalize(join(dirname(from), spec));
  if (existsSync(base) && statSync(base).isFile()) return base;
  for (const ext of [".js", "/index.js"]) if (existsSync(base + ext)) return normalize(base + ext);
  fail(`cannot resolve "${spec}" from ${from}`);
}

const ordered = [];
const state = new Map();

function visit(file, stack) {
  const mark = state.get(file);
  if (mark === "done") return;
  if (mark === "visiting") fail(`import cycle: ${[...stack, file].join(" -> ")}`);
  state.set(file, "visiting");

  for (const { spec } of importsOf(file)) {
    if (spec.startsWith("node:")) continue;
    if (!spec.startsWith(".")) fail(`${file} imports external package "${spec}"`);
    visit(resolveLocal(file, spec), [...stack, file]);
  }

  state.set(file, "done");
  ordered.push(file);
}

visit(ENTRY, []);

/* --------------------------------------------------------------------------
 * 2. Per-module analysis
 * ------------------------------------------------------------------------ */

/** Names this module exports, and whether it uses `export default`. */
function analyse(file) {
  let code = readFileSync(file, "utf8");
  if (code.charCodeAt(0) === 0xfeff) code = code.slice(1);

  if (/export\s*\{[^}]*\}\s*from/.test(code)) fail(`${file} uses "export {} from"`);
  if (/export\s*\{/.test(code)) fail(`${file} uses an export list, which is not named here`);

  const named = new Set();
  const re = /^[ \t]*export\s+(?:async\s+)?(?:const|let|var|function|class)\s+([A-Za-z_$][\w$]*)/gm;
  let m;
  while ((m = re.exec(code))) named.add(m[1]);

  const hasDefault = /^[ \t]*export\s+default\b/m.test(code);
  if (hasDefault) {
    /* Only `export default Rule` and `export default { ... }` occur. A default
       with any other shape would need a real parser to handle. */
    const namedForm = /^[ \t]*export\s+default\s+([A-Za-z_$][\w$]*)\s*;?[ \t]*$/m.exec(code);
    if (!namedForm) {
      const inline = /^[ \t]*export\s+default\s*(\{|\()/m.test(code);
      if (!inline) fail(`${file} has an unrecognised export default`);
    }
    named.add("default");
  }

  return { named, hasDefault };
}

/* --------------------------------------------------------------------------
 * 3. Emit
 * ------------------------------------------------------------------------ */

/*
 * Each node builtin the engine may use, with the local name its namespace is
 * required under. The *members* are NOT listed here any more: the import
 * rewrite below binds exactly the names a module asked for.
 *
 * A hand-maintained member list used to decide this, and it silently dropped
 * any name it did not know: `import { basename, extname } from "node:path"`
 * produced a bundle where `extname` was never declared. Nothing failed at build
 * time -- `scripts/smoke-bundle.mjs` tested the CLI, and the broken call was
 * only reachable over the RPC path the desktop app uses, so the failure was
 * waiting for the first user to open a plan file. Deriving the bindings from the
 * imports removes the list that can fall out of date.
 */
const BUILTINS = [
  { binding: "fs", module: "node:fs" },
  { binding: "fsp", module: "node:fs/promises" },
  { binding: "path", module: "node:path" },
  { binding: "crypto", module: "node:crypto" },
  { binding: "url", module: "node:url" },
  { binding: "os", module: "node:os" },
  { binding: "http", module: "node:http" },
  { binding: "child", module: "node:child_process" },
];

const BUILTIN_BY_MODULE = new Map(BUILTINS.map((builtin) => [builtin.module, builtin]));

/** Builtins an import rewrite actually referenced, in first-seen order. */
const usedBuiltins = new Map();

const id = (file) => posix(file).replace(/[^A-Za-z0-9]/g, "_");
const analysis = new Map(ordered.map((f) => [f, analyse(f)]));

/* Namespaces are declared first so a module body can reference a sibling that
   appears later in the emitted order. */
const namespaceDecls = ordered.map((f) => `var ${id(f)} = {};`).join("\n");

const moduleChunks = [];
const allBodies = [];

for (const file of ordered) {
  const { named } = analysis.get(file);
  let code = readFileSync(file, "utf8");
  if (code.charCodeAt(0) === 0xfeff) code = code.slice(1);

  const ns = id(file);
  const notes = [];

  /* Version is inlined: a standalone binary has no package.json beside it. */
  if (posix(file) === "src/version.js") {
    code = `export const PRODUCT = "BauLens";
export const VENDOR = "Keller Systems";
export const VERSION = ${JSON.stringify(VERSION)};
export const PRODUCT_LABEL = PRODUCT + " v" + VERSION;
export const ENGINE_LABEL = PRODUCT + " Core v" + VERSION;
`;
    notes.push("version inlined");
  }

  /* Shebang out. */
  code = code.replace(/^#![^\n]*\n/, "");

  /* Imports -> reads from the sibling namespace. Three shapes occur:
       import { a, b as c } from "./x.js"
       import thing        from "./x.js"     (the default export)
       import "./x.js"                        (bare, for side effects)      */
  code = code.replace(
    /^[ \t]*import\s+([\s\S]*?)\s+from\s+["']([^"']+)["'];?[ \t]*$/gm,
    (whole, clause, spec) => {
      /* A builtin import becomes a destructuring from the namespace the preamble
         required, so every name the module imported is bound -- including names
         nobody anticipated at the time this script was written. */
      if (spec.startsWith("node:")) {
        const builtin = BUILTIN_BY_MODULE.get(spec);
        if (!builtin) fail(`${file} imports ${spec}, which the builtin preamble does not declare`);
        usedBuiltins.set(builtin.module, file);
        const inner = /^\{(.*)\}$/.exec(clause.trim());
        if (!inner) fail(`${file} imports ${spec} in a form the bundler cannot read: "${clause}"`);
        const specifiers = [];
        for (const pair of inner[1].split(",")) {
          const part = pair.trim();
          if (!part) continue;
          const asMatch = /^([A-Za-z_$][\w$]*)\s+as\s+([A-Za-z_$][\w$]*)$/.exec(part);
          if (asMatch) specifiers.push(`${asMatch[1]}: ${asMatch[2]}`);
          else if (/^[A-Za-z_$][\w$]*$/.test(part)) specifiers.push(part);
          else fail(`${file} has an unreadable import specifier "${part}" from ${spec}`);
        }
        return specifiers.length ? `    var { ${specifiers.join(", ")} } = ${builtin.binding};` : "";
      }
      const target = id(resolveLocal(file, spec));
      clause = clause.trim();

      if (/^\*\s+as\s/.test(clause)) fail(`${file} uses a namespace import`);

      const lines = [];

      /* Default import: `thing` or `thing, { a }`.

         The brace patterns use `[\s\S]` rather than `.` because an import list
         may be written over several lines, and `.` never matches a line break -
         a multi-line `import {\n a,\n b\n}` used to fail here as "unreadable",
         which is a legal module rejected by the bundler rather than a mistake in
         the module. */
      const defaultMatch = /^([A-Za-z_$][\w$]*)\s*(?:,\s*([\s\S]+))?$/.exec(clause);
      const bracePart = /^\{([\s\S]*)\}$/;
      if (defaultMatch && !bracePart.test(clause) && !clause.startsWith("{")) {
        lines.push(`    var ${defaultMatch[1]} = ${target}.default;`);
        if (defaultMatch[2]) clause = defaultMatch[2].trim();
        else clause = "";
      }

      if (clause) {
        const inner = bracePart.exec(clause);
        if (!inner) fail(`${file} has an import clause the bundler cannot read: "${clause}"`);
        for (const pair of inner[1].split(",")) {
          const part = pair.trim();
          if (!part) continue;
          const asMatch = /^([A-Za-z_$][\w$]*)\s+as\s+([A-Za-z_$][\w$]*)$/.exec(part);
          if (asMatch) lines.push(`    var ${asMatch[2]} = ${target}.${asMatch[1]};`);
          else if (/^[A-Za-z_$][\w$]*$/.test(part)) lines.push(`    var ${part} = ${target}.${part};`);
          else fail(`${file} has an unreadable import specifier "${part}"`);
        }
      }

      return lines.join("\n");
    }
  );

  /* Bare imports only exist for side effects; there are none in this codebase. */
  code = code.replace(/^[ \t]*import\s+["'][^"']+["'];?[ \t]*$/gm, "");

  const exported = [...named].filter((n) => n !== "default");

  /* Top-level await is legal in an ES module and illegal in CommonJS, which is
     what the bundle becomes. The entry point uses it to await main(), so that
     one call is wrapped in an async IIFE. Any other top-level await means the
     module has statements the wrapper would not cover, and the bundle would
     produce a syntax error at run time rather than here. */
  if (/^[ \t]*await\b/m.test(code)) {
    const awaitedCall = /^[ \t]*const\s+(\w+)\s*=\s*await\s+([\s\S]*?);[ \t]*$/m.exec(code);
    if (!awaitedCall) {
      fail(
        `${file} uses top-level await in a form the bundler cannot wrap. ` +
        `Move it into a named async function.`
      );
    }
    const [whole, binding, call] = awaitedCall;
    const line = code.slice(0, awaitedCall.index).split("\n").length;

    /* A following `process.exit(binding);` belongs inside the wrapper too.
       Leaving it outside referenced a binding scoped to the IIFE and threw a
       ReferenceError after the CLI had already produced its output. */
    const trailing = new RegExp(
      `^[ \\t]*process\\.exit\\(\\s*${binding}\\s*\\);[ \\t]*$`,
      "m"
    );
    const hadTrailing = trailing.test(code);
    if (hadTrailing) code = code.replace(trailing, "");

    code = code.replace(
      whole,
      `(async () => {\n  const ${binding} = await ${call.trim()};\n  process.exit(${binding});\n})();`
    );
    notes.push(`top-level await wrapped${hadTrailing ? " with its exit call" : ""}`);
  }

  /* Named default: bind the alias so the registry can refer to it, and expose
     it as `default` for anything that imported it by that name. */
  let defaultAlias = null;
  if (analysis.get(file).hasDefault) {
    defaultAlias = `Rule_${posix(file).split("/").pop().replace(/\.js$/, "")}`;
    const namedForm = /^[ \t]*export\s+default\s+([A-Za-z_$][\w$]*)\s*;?[ \t]*$/m.exec(code);
    if (namedForm) {
      code = code.replace(namedForm[0], `const ${defaultAlias} = ${namedForm[1]};`);
    } else {
      code = code.replace(/^[ \t]*export\s+default\s*/m, `const ${defaultAlias} = `);
    }
    notes.push(`default -> ${defaultAlias}`);
  }

  /* Public surface onto the namespace object, then strip the export keywords.
     The entry point is a script and exports nothing, which is expected. */
  const assigns = exported.map((n) => `  ${ns}.${n} = ${n};`);
  if (defaultAlias) assigns.push(`  ${ns}.default = ${defaultAlias};`);
  if (!assigns.length && posix(file) !== posix(ENTRY)) fail(`${file} exports nothing`);

  code = code
    .replace(/^[ \t]*export\s+(?:async\s+)?(?:const|let|var|function|class)\s+/gm, (s) => s.replace(/\bexport\s+/, ""))
    .trim();

  const body = `${code}\n\n${assigns.join("\n")}`;
  allBodies.push(body);

  /* The body is emitted at its original indentation. Re-indenting each line to
     sit inside the function wrapper would look tidier and is WRONG: it rewrites
     the contents of template literals and multi-line strings, which changes user
     visible output. That is how the --help text lost its leading spaces. */
  moduleChunks.push(
    `/* ===== ${posix(file)}${notes.length ? "  [" + notes.join(", ") + "]" : ""} ===== */\n` +
    `function ${ns}_init() {\n${body}\n}`
  );
}

/* --------------------------------------------------------------------------
 * 4. Runtime preamble: only the builtins actually used
 * ------------------------------------------------------------------------ */

/* Only the builtins a module actually imported are required. Recorded during the
   rewrite rather than guessed from the emitted text: a substring search for
   "url" also matches the word in a comment, and a require that is never needed
   is noise in a file whose whole point is to be inspectable. */
const requires = [];
const builtinNames = [...usedBuiltins.keys()].sort();

for (const builtin of BUILTINS) {
  if (!usedBuiltins.has(builtin.module)) continue;
  requires.push(`const ${builtin.binding} = require("${builtin.module}");`);
}

/* `process`, `Buffer`, `console` and timers are CommonJS globals; declaring
   them here would shadow the real objects for every module at once. */

const banner = `#!/usr/bin/env node
/**
 * BauLens v${VERSION} - single-file build (${ordered.length} modules).
 *
 * GENERATED by scripts/bundle.js. Do not edit; every change belongs in src/.
 * Deterministic rule engine for DACH building-code pre-checks.
 * Vorpruefung, keine Rechtsberatung.
 */
"use strict";

${requires.join("\n")}

/* Forward declarations, so any module body may reference a sibling. */
${namespaceDecls}

/* Initialise every module once, dependencies first (the order below already
   guarantees a dependency is initialised before its dependents). */
`;

const initCalls = ordered.map((f) => `${id(f)}_init();`).join("\n");

const footer = `
/* Entry point last: it runs the CLI. */
`;

mkdirSync(OUT_DIR, { recursive: true });
writeFileSync(
  OUT_FILE,
  `${banner}\n${moduleChunks.join("\n\n")}\n\n${initCalls}\n${footer}`,
  "utf8"
);

/* --------------------------------------------------------------------------
 * 5. Report and self-checks
 * ------------------------------------------------------------------------ */

const bytes = readFileSync(OUT_FILE).length;
const rules = ordered.filter((f) => posix(f).startsWith("src/rules/") && !f.endsWith("index.js"));

if (rules.length !== 15) fail(`expected 15 rule modules, found ${rules.length}`);
if (ordered[ordered.length - 1] !== ENTRY) fail("entry module is not emitted last");

console.log(`bundled ${ordered.length} modules -> ${OUT_FILE}`);
console.log(`  version        : ${VERSION} (inlined, no package.json needed)`);
console.log(`  scoped modules : ${ordered.length} (no shared top-level scope)`);
console.log(`  rule modules   : ${rules.length}`);
console.log(`  node builtins  : ${builtinNames.join(", ")}`);
console.log(`  size           : ${(bytes / 1024).toFixed(1)} KiB`);

const leftover = readFileSync(OUT_FILE, "utf8").match(/^\s*(import|export)\b/gm);
if (leftover) fail(`${leftover.length} import/export statement(s) survived`);
