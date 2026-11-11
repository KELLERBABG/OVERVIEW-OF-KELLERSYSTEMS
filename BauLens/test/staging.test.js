/**
 * Staging: the desktop injection must land at the document end as its own
 * script element — never inside a page's own script source.
 *
 * studio.html embeds a whole HTML document in a template literal (the dossier
 * export), including a `</body>`. The build used to splice at the first
 * occurrence, which put the injected script inside that string: its
 * `</script>` truncated the page's script element mid-template, so neither
 * the page code nor the gate ever ran in the packaged app. This test builds
 * the staged pages and pins the parse shape the webview sees.
 */
import { test } from "node:test";
import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const root = dirname(dirname(fileURLToPath(import.meta.url)));

test("Staging: injection is its own script element at the document end", () => {
  const build = spawnSync("node", ["scripts/build-desktop-assets.mjs"], {
    cwd: root,
    encoding: "utf8",
  });
  if (build.error) throw build.error;
  assert.equal(build.status, 0, "build-desktop-assets failed:\n" + build.stderr);

  for (const page of ["index.html", "studio.html"]) {
    const html = readFileSync(join(root, "desktop", "app", page), "utf8");

    /* Strict alternation: every <script> must close before the next one
       opens. A second open between an open and its close means the
       injection sits inside the page's own source and would truncate it. */
    const tokens = [...html.matchAll(/<\/?script[^>]*>/g)].map((m) => m[0]);
    assert.ok(tokens.length >= 2, page + ": page script plus injection expected");
    tokens.forEach((token, i) => {
      assert.equal(
        token.startsWith("<script"),
        i % 2 === 0,
        page + ": token " + i + " out of order: " + token.slice(0, 48)
      );
    });

    /* The injection marker lives after the page's own script has closed. */
    const marker = html.indexOf("<!-- __baulensDesktopInjected -->");
    assert.ok(marker !== -1, page + ": injection marker missing");
    assert.ok(
      marker > html.indexOf("</scr" + "ipt>"),
      page + ": injection must come after the page's own script"
    );

    /* And it is the last thing before the document end. */
    assert.match(
      html.slice(-64),
      /<\/script>\s*<\/body>\s*<\/html>\s*$/,
      page + ": injection must sit right before </body>"
    );
  }
});
