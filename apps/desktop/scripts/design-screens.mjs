#!/usr/bin/env node
/* global process, console, document, localStorage */
/**
 * Screenshots of every section of the app (mock backend, 1280x800, both themes) for the
 * design project: PNGs in design/schermate/ plus one @dsCard HTML per screen, which
 * /design-sync uploads with the rest of design/. Needs `VITE_MOCK=1 pnpm dev` running.
 *
 * Usage: node scripts/design-screens.mjs [--url http://localhost:1420] [--out design/schermate] [--chromium <path>]
 */

import fs from "node:fs";
import path from "node:path";
import { chromium } from "playwright";

const args = process.argv.slice(2);
const getArg = (name, fallback) => {
  const idx = args.indexOf(name);
  return idx >= 0 && args[idx + 1] ? args[idx + 1] : fallback;
};
const BASE_URL = getArg("--url", "http://localhost:1420");
const OUT_DIR = path.resolve(getArg("--out", "design/schermate"));
const CHROMIUM_PATH = getArg("--chromium", process.env.PLAYWRIGHT_CHROMIUM || null);

const THEMES = [
  { id: "forge", label: "Fucina" },
  { id: "light", label: "Pergamena" },
];

const nav = (page, section) => page.locator(`button[data-nav-section="${section}"], button[data-section="${section}"]`).first().click();
const labTab = (page, name) => page.getByRole("tab", { name }).or(page.locator(`button:has-text("${name}")`)).first().click();
const labMore = async (page, name) => {
  await page.locator('button:has-text("Altro")').or(page.locator('button:has(span:text-is("Altro"))')).first().click();
  await page.locator(`button:has-text("${name}")`).last().click();
};

// Each screen: file slug, card title, what it shows, and the steps that bring the app there.
const SCREENS = [
  { slug: "panoramica", title: "Panoramica", note: "Home: deck recenti, novità e scorciatoie.", go: (p) => nav(p, "overview") },
  { slug: "fucina", title: "Fucina (deck builder)", note: "Deck, Extra, Side e Maybe board a sinistra, catalogo a destra.", go: (p) => nav(p, "deck-builder") },
  {
    slug: "fucina-ispettore",
    title: "Fucina con il pannello della carta",
    note: "Una carta del Main Deck fissata nell'Ispettore: testo, ruoli e legalità.",
    go: async (p) => {
      await nav(p, "deck-builder");
      await p.waitForTimeout(500);
      await p.locator("button[data-rarity]").nth(3).click();
    },
  },
  { slug: "scaffale", title: "Scaffale", note: "Gli scaffali 3D con le deck box.", go: (p) => nav(p, "shelf") },
  { slug: "lab-panoramica", title: "Laboratorio: Panoramica", note: "Probabilità esatte sui ruoli delle carte.", go: async (p) => { await nav(p, "stats"); await labTab(p, "Panoramica"); } },
  { slug: "lab-collegamenti", title: "Laboratorio: Collegamenti", note: "Mappa circolare dei collegamenti tra le carte.", go: async (p) => { await nav(p, "stats"); await labTab(p, "Collegamenti"); } },
  { slug: "lab-confronto", title: "Laboratorio: Confronto", note: "Confronto tra due versioni del deck.", go: async (p) => { await nav(p, "stats"); await labTab(p, "Confronto"); } },
  { slug: "lab-calcolatore", title: "Laboratorio: Calcolatore", note: "Probabilità di pescare le carte volute.", go: async (p) => { await nav(p, "stats"); await labTab(p, "Calcolatore"); } },
  { slug: "lab-carte-speciali", title: "Laboratorio: Carte speciali", note: "Calcolatori per pescate, scavi, ponti e ricerche verso un bersaglio.", go: async (p) => { await nav(p, "stats"); await labTab(p, "Carte speciali"); await p.locator('button:has-text("Pot of Desires")').first().click(); } },
  { slug: "lab-simulatore", title: "Laboratorio: Simulatore", note: "Mani simulate e statistiche.", go: async (p) => { await nav(p, "stats"); await labMore(p, "Simulatore"); } },
  { slug: "lab-ottimizzatore", title: "Laboratorio: Ottimizzatore", note: "Ricerca della composizione migliore.", go: async (p) => { await nav(p, "stats"); await labMore(p, "Ottimizzatore"); } },
  { slug: "lab-contro-il-meta", title: "Laboratorio: Contro il meta", note: "Tech table: quali handtrap fermano più linee dei deck più giocati.", go: async (p) => { await nav(p, "stats"); await labMore(p, "Contro il meta"); await p.getByRole("button", { name: /^Calcola$/ }).click(); await p.waitForTimeout(1500); } },
  { slug: "lab-registro", title: "Laboratorio: Registro partite", note: "Le partite registrate.", go: async (p) => { await nav(p, "stats"); await labMore(p, "Registro partite"); } },
  { slug: "combo-impostazioni", title: "Combo Lab: impostazioni", note: "Mano, obiettivo e parametri della ricerca.", go: (p) => nav(p, "combo") },
  {
    slug: "combo-risultati",
    title: "Combo Lab: risultati",
    note: "Le board trovate, ordinate per punteggio.",
    go: async (p) => {
      await nav(p, "combo");
      await p.getByRole("button", { name: /Cerca le combo/i }).first().click();
      await p.waitForTimeout(1200);
    },
  },
  {
    slug: "combo-dettaglio",
    title: "Combo Lab: dettaglio di una board",
    note: "La linea di gioco passo per passo, con il punteggio.",
    go: async (p) => {
      await nav(p, "combo");
      await p.getByRole("button", { name: /Cerca le combo/i }).first().click();
      await p.waitForTimeout(1200);
      await p.locator('div[role="button"]:has(span.score-bar)').first().click();
      await p.waitForTimeout(500);
    },
  },
  { slug: "banditore", title: "Il Banditore", note: "Novità, banlist e prossime uscite.", go: (p) => nav(p, "news") },
  { slug: "impostazioni", title: "Impostazioni", note: "Dati, aspetto, suoni.", go: (p) => nav(p, "settings") },
];

const card = (s, theme) => `<!-- @dsCard group="Schermate" -->
<!doctype html>
<html lang="it" data-theme="${theme.id}">
<head>
<meta charset="utf-8">
<title>${s.title} (${theme.label})</title>
<style>html,body{margin:0;background:#000}img{display:block;width:1280px;height:800px}</style>
</head>
<body><img src="${s.slug}-${theme.id}.png" alt="${s.title}, tema ${theme.label}. ${s.note}"></body>
</html>
`;

async function main() {
  fs.mkdirSync(OUT_DIR, { recursive: true });
  const browser = await chromium.launch({ headless: true, ...(CHROMIUM_PATH ? { executablePath: CHROMIUM_PATH } : {}) });
  const problems = [];
  try {
    const context = await browser.newContext({ viewport: { width: 1280, height: 800 } });
    const page = await context.newPage();
    page.setDefaultTimeout(8000);
    page.on("pageerror", (e) => problems.push(`pageerror: ${e.message}`));
    await page.goto(BASE_URL, { waitUntil: "networkidle" });
    for (const theme of THEMES) {
      await page.evaluate(([t]) => {
        document.documentElement.setAttribute("data-theme", t);
        try {
          localStorage.setItem("df_theme", t);
          localStorage.setItem("duelforge.lastDeck", "2");
        } catch {
          // ignore
        }
      }, [theme.id]);
      for (const s of SCREENS) {
        try {
          await page.reload({ waitUntil: "load" });
          await page.waitForTimeout(800);
          await page.locator('button:has-text("Inizia"), button:has-text("Chiudi"), [aria-label="Chiudi"]').first().click({ timeout: 1500 }).catch(() => {});
          await page.evaluate((t) => document.documentElement.setAttribute("data-theme", t), theme.id);
          await s.go(page);
          await page.waitForTimeout(700);
          await page.screenshot({ path: path.join(OUT_DIR, `${s.slug}-${theme.id}.png`) });
          fs.writeFileSync(path.join(OUT_DIR, `${s.slug}-${theme.id}.html`), card(s, theme));
          console.log(`✓ ${s.slug}-${theme.id}`);
        } catch (e) {
          problems.push(`${s.slug}-${theme.id}: ${e.message.split("\n")[0]}`);
        }
      }
    }
  } finally {
    await browser.close();
  }
  if (problems.length) {
    console.error(`\nProblemi:\n${problems.map((p) => `  - ${p}`).join("\n")}`);
    process.exit(1);
  }
}

main();
