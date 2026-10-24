#!/usr/bin/env node
/* global process, console, document, URL */
/**
 * Exports the component gallery (#/galleria, mock mode) as the design-system
 * previews in apps/desktop/design/: one standalone HTML per [data-ds-card],
 * starting with the `<!-- @dsCard group="…" -->` marker that /design-sync reads,
 * plus the app's compiled stylesheet (duelforge.css), its fonts and textures,
 * and an index.html with every card on one page.
 *
 * Usage:
 *   node scripts/design-export.mjs [--out design] [--shots <dir>] [--artifact <dir>] [--chromium <path>]
 *
 * --shots also saves a PNG of each card from the live gallery, to check the
 * previews by eye. --artifact writes the page published as the Artifact preview
 * (index.html as a fragment, duelforge.css, assets/). The previews are generated: edit the gallery, not the HTML.
 */

import fs from "node:fs";
import http from "node:http";
import os from "node:os";
import path from "node:path";
import { execFileSync } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const args = process.argv.slice(2);
const getArg = (name, fallback) => {
  const idx = args.indexOf(name);
  return idx >= 0 && args[idx + 1] ? args[idx + 1] : fallback;
};

const APP_DIR = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const OUT_DIR = path.resolve(APP_DIR, getArg("--out", "design"));
const SHOTS_DIR = getArg("--shots", null);
const ARTIFACT_DIR = getArg("--artifact", null);
const CHROMIUM_PATH = getArg("--chromium", process.env.PLAYWRIGHT_CHROMIUM || null);
const BUILD_DIR = path.join(os.tmpdir(), "duelforge-design-build");

const MIME = {
  ".html": "text/html",
  ".js": "text/javascript",
  ".css": "text/css",
  ".woff2": "font/woff2",
  ".woff": "font/woff",
  ".webp": "image/webp",
  ".svg": "image/svg+xml",
  ".png": "image/png",
};

/** Vite names assets `name-<8 char hash>.ext`: drop the hash so the files keep their names between exports. */
const stableName = (file) => path.basename(file).replace(/-[\w-]{8}(\.\w+)$/, "$1");

const slug = (s) =>
  s
    .normalize("NFD")
    .replace(/[̀-ͯ]/g, "")
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-|-$/g, "");

const escapeHtml = (s) => s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

function build() {
  console.log("Building the mock frontend...");
  fs.rmSync(BUILD_DIR, { recursive: true, force: true });
  execFileSync("pnpm", ["exec", "vite", "build", "--base", "./", "--outDir", BUILD_DIR, "--emptyOutDir", "--logLevel", "warn"], {
    cwd: APP_DIR,
    stdio: "inherit",
    env: { ...process.env, VITE_MOCK: "1" },
  });
}

function serve() {
  const server = http.createServer((req, res) => {
    const url = new URL(req.url, "http://localhost");
    const file = path.join(BUILD_DIR, url.pathname === "/" ? "index.html" : decodeURIComponent(url.pathname));
    if (!file.startsWith(BUILD_DIR) || !fs.existsSync(file) || fs.statSync(file).isDirectory()) {
      res.writeHead(404);
      res.end();
      return;
    }
    res.writeHead(200, { "Content-Type": MIME[path.extname(file)] ?? "application/octet-stream" });
    fs.createReadStream(file).pipe(res);
  });
  return new Promise((resolve) => server.listen(0, "127.0.0.1", () => resolve(server)));
}

/**
 * The compiled stylesheet, with only the latin font subsets (Italian and the
 * card names need nothing else) and every url() pointing to assets/.
 */
function exportStyles() {
  const assetsDir = path.join(BUILD_DIR, "assets");
  const css = fs
    .readdirSync(assetsDir)
    .filter((f) => f.endsWith(".css"))
    .sort()
    .map((f) => fs.readFileSync(path.join(assetsDir, f), "utf8"))
    .join("\n");
  const fontFaces = css.replace(/@font-face\{[^}]*\}/g, (block) => (/-latin-(?!ext)/.test(block) ? block : ""));
  const copied = new Set();
  const out = fontFaces.replace(/url\(\s*(['"]?)([^'")]+)\1\s*\)/g, (match, _q, ref) => {
    if (ref.startsWith("data:") || ref.startsWith("#")) return match;
    const source = path.join(assetsDir, path.basename(ref.split(/[?#]/)[0]));
    if (!fs.existsSync(source)) return match;
    const name = stableName(source);
    if (!copied.has(name)) {
      fs.copyFileSync(source, path.join(OUT_DIR, "assets", name));
      copied.add(name);
    }
    return `url(assets/${name})`;
  });
  fs.writeFileSync(path.join(OUT_DIR, "duelforge.css"), out);
  return copied.size;
}

/** A standalone preview: the marker on the first line, the shared stylesheet, the card's markup. */
function previewPage({ group, name, html, width }, depth) {
  const up = "../".repeat(depth);
  return `<!-- @dsCard group="${escapeHtml(group)}" -->
<!doctype html>
<html lang="it" data-theme="forge">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(name)}</title>
<link rel="stylesheet" href="${up}duelforge.css">
<style>html,body{height:auto;overflow:visible}body{padding:16px}.ds-frame{width:${width}px;max-width:100%}</style>
</head>
<body>
<div class="ds-frame">
${html}
</div>
</body>
</html>
`;
}

/**
 * Every card on one page. As a full document for design/index.html; with
 * `fragment` it is the body of the Artifact preview, which the Artifact
 * wraps in its own document (title and meta tags go first).
 */
function indexPage(cards, fragment = false) {
  const sections = cards
    .map(
      (c) => `<section class="flex flex-col gap-2" id="${c.id}">
<div class="flex flex-wrap items-baseline gap-x-3 px-1"><h2 class="text-title text-text">${escapeHtml(c.name)}</h2><span class="eyebrow">${escapeHtml(c.group)}</span>${
        c.note ? `<span class="text-small text-faint">${escapeHtml(c.note)}</span>` : ""
      }</div>
${c.html}
</section>`,
    )
    .join("\n");
  // The previews keep the app's layout; on a phone the two themes stack and wide rows scroll inside their pane.
  const style = `<style>html,body{height:auto;overflow:visible}
.ds-index{padding-block:32px;padding-left:max(16px,env(safe-area-inset-left,0px));padding-right:max(16px,env(safe-area-inset-right,0px))}
.ds-pair>div{overflow-x:auto}
@media (max-width:900px){.ds-pair{grid-template-columns:1fr}}</style>`;
  const head = `<title>Sistema di design DuelForge</title>
<meta name="claude:viewport-fit-sides" content="cover">
<link rel="stylesheet" href="duelforge.css">
${style}`;
  const body = `<main class="ds-index mx-auto flex max-w-[1500px] flex-col gap-10">
<header><p class="eyebrow">Generata da scripts/design-export.mjs</p><h1 class="display text-2xl font-bold">Sistema di design DuelForge</h1>
<p class="text-small text-muted">Ogni scheda mostra un gruppo di token o un componente nei due temi: Fucina (scuro) e Pergamena (chiaro).</p></header>
${sections}
</main>`;
  if (fragment) return `${head}\n${body}\n`;
  return `<!doctype html>
<html lang="it" data-theme="forge">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
${head}
</head>
<body>
${body}
</body>
</html>
`;
}

async function main() {
  build();
  const server = await serve();
  const base = `http://127.0.0.1:${server.address().port}`;
  const browser = await chromium.launch({ headless: true, ...(CHROMIUM_PATH ? { executablePath: CHROMIUM_PATH } : {}) });
  try {
    // Tall enough that lazy card images are all in view and load.
    const page = await browser.newPage({ viewport: { width: 1560, height: 6000 } });
    page.on("pageerror", (e) => console.error(`Page error: ${e.message}`));
    await page.goto(`${base}/#/galleria`, { waitUntil: "load" });
    await page.waitForSelector("[data-gallery-ready]", { timeout: 20000 });
    // Card art and textures, then let the entry animations finish.
    await page.waitForFunction(() => [...document.images].every((img) => img.complete), null, { timeout: 15000 }).catch(() => {
      console.warn("Some images did not load in 15 s: exporting anyway.");
    });
    await page.waitForTimeout(1200);

    const cards = await page.$$eval("[data-ds-card]", (els) =>
      els.map((el) => {
        const frame = el.children[1];
        return {
          id: el.getAttribute("data-ds-card"),
          group: el.getAttribute("data-ds-group"),
          name: el.getAttribute("data-ds-name"),
          note: el.querySelector(":scope > div > span.text-faint")?.textContent ?? "",
          html: frame.outerHTML,
          width: Math.ceil(frame.getBoundingClientRect().width),
        };
      }),
    );

    // Only generated files live here: start from an empty folder so removed cards disappear.
    for (const entry of fs.existsSync(OUT_DIR) ? fs.readdirSync(OUT_DIR) : []) {
      if (entry !== "README.md") fs.rmSync(path.join(OUT_DIR, entry), { recursive: true, force: true });
    }
    fs.mkdirSync(path.join(OUT_DIR, "assets"), { recursive: true });
    const assets = exportStyles();

    for (const card of cards) {
      const dir = path.join(OUT_DIR, slug(card.group));
      fs.mkdirSync(dir, { recursive: true });
      fs.writeFileSync(path.join(dir, `${card.id}.html`), previewPage(card, 1));
    }
    fs.writeFileSync(path.join(OUT_DIR, "index.html"), indexPage(cards));

    if (ARTIFACT_DIR) {
      // The Artifact preview: the index as a fragment, next to the stylesheet and its assets.
      fs.rmSync(ARTIFACT_DIR, { recursive: true, force: true });
      fs.mkdirSync(ARTIFACT_DIR, { recursive: true });
      fs.cpSync(path.join(OUT_DIR, "assets"), path.join(ARTIFACT_DIR, "assets"), { recursive: true });
      fs.copyFileSync(path.join(OUT_DIR, "duelforge.css"), path.join(ARTIFACT_DIR, "duelforge.css"));
      fs.writeFileSync(path.join(ARTIFACT_DIR, "index.html"), indexPage(cards, true));
    }

    if (SHOTS_DIR) {
      fs.mkdirSync(SHOTS_DIR, { recursive: true });
      for (const card of cards) {
        await page.locator(`[data-ds-card="${card.id}"]`).screenshot({ path: path.join(SHOTS_DIR, `${card.id}.png`) });
      }
      // The standalone index, to check that the previews render without the app.
      const preview = await browser.newPage({ viewport: { width: 1560, height: 1000 } });
      await preview.goto(`file://${path.join(OUT_DIR, "index.html")}`, { waitUntil: "load" });
      await preview.screenshot({ path: path.join(SHOTS_DIR, "_index.png"), fullPage: true });
    }

    console.log(`${cards.length} previews and ${assets} assets in ${path.relative(process.cwd(), OUT_DIR) || "."}`);
  } finally {
    await browser.close();
    server.close();
  }
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
