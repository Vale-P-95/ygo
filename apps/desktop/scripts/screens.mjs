#!/usr/bin/env node
/* global process, console, document, localStorage */
/**
 * Screenshot and accessibility audit tool using Playwright & @axe-core/playwright.
 * Principle: captures UI states at 1280x800 and 1920x1080 in both "forge" and "light" themes.
 *
 * Usage:
 *   node scripts/screens.mjs [--url http://localhost:5173] [--out screenshots] [--axe]
 */

import fs from "node:fs";
import path from "node:path";
import { chromium } from "playwright";
import AxeBuilder from "@axe-core/playwright";

const args = process.argv.slice(2);
const getArg = (name, fallback) => {
  const idx = args.indexOf(name);
  return idx >= 0 && args[idx + 1] ? args[idx + 1] : fallback;
};
const hasArg = (name) => args.includes(name);

const BASE_URL = getArg("--url", "http://localhost:5173");
const OUT_DIR = path.resolve(getArg("--out", path.join(process.cwd(), "screenshots")));
const RUN_AXE = hasArg("--axe");

const VIEWPORTS = [
  { width: 1280, height: 800, name: "1280x800" },
  { width: 1440, height: 900, name: "1440x900" },
  { width: 1680, height: 1050, name: "1680x1050" },
  { width: 1920, height: 1080, name: "1920x1080" },
  { width: 1280, height: 600, name: "1280x600" },
];

const THEMES = ["forge", "light"];

const SECTIONS = [
  { id: "overview", name: "panoramica" },
  { id: "deck-builder", name: "fucina" },
  { id: "shelf", name: "bottega" },
  { id: "stats", name: "laboratorio" },
  { id: "combo", name: "circolo" },
  { id: "news", name: "banditore" },
  { id: "settings", name: "impostazioni" },
];

async function main() {
  if (!fs.existsSync(OUT_DIR)) {
    fs.mkdirSync(OUT_DIR, { recursive: true });
  }

  console.log(`Connecting to ${BASE_URL}...`);
  const browser = await chromium.launch({ headless: true });

  let totalScreenshots = 0;
  let totalAxeViolations = 0;

  try {
    for (const vp of VIEWPORTS) {
      console.log(`\n--- Viewport ${vp.name} ---`);
      const context = await browser.newContext({
        viewport: { width: vp.width, height: vp.height },
      });
      const page = await context.newPage();

      // Load initial page with mock mode
      await page.goto(BASE_URL, { waitUntil: "networkidle" });

      for (const theme of THEMES) {
        console.log(`  Tema: ${theme}`);
        await page.evaluate((t) => {
          document.documentElement.setAttribute("data-theme", t);
          try {
            localStorage.setItem("df_theme", t);
          } catch {
            // ignore
          }
        }, theme);

        await page.waitForTimeout(300);

        for (const section of SECTIONS) {
          // Switch section by clicking sidebar button or via DOM dispatch
          const switched = await page.evaluate((secId) => {
            const btn = document.querySelector(`button[data-nav-section="${secId}"]`) ||
                        Array.from(document.querySelectorAll("button")).find((b) => b.getAttribute("data-section") === secId);
            if (btn) {
              btn.click();
              return true;
            }
            return false;
          }, section.id);

          if (!switched) {
            // Fallback: click section by title or text
            const navBtn = page.locator(`button:has-text("${section.name}")`).first();
            if (await navBtn.count()) {
              await navBtn.click();
            }
          }

          if (section.id === "stats") {
            const linksTab = page.locator('button:has-text("Collegamenti")').first();
            if (await linksTab.count()) {
              await linksTab.click();
            }
          }

          await page.waitForTimeout(400);

          const fileName = `${theme}_${vp.name}_${section.id}.png`;
          const filePath = path.join(OUT_DIR, fileName);
          await page.screenshot({ path: filePath, fullPage: false });
          totalScreenshots++;
          console.log(`    ✓ ${fileName}`);

          if (RUN_AXE) {
            try {
              const results = await new AxeBuilder({ page })
                .withTags(["wcag2a", "wcag2aa", "wcag21a", "wcag21aa"])
                .disableRules(["color-contrast"]) // Optional fine-tuning if custom canvas/webgl is present
                .analyze();

              if (results.violations.length > 0) {
                console.warn(`    ⚠ Axe: ${results.violations.length} violazioni in ${section.id} (${theme})`);
                totalAxeViolations += results.violations.length;
              } else {
                console.log(`    ✓ Axe OK: 0 violazioni`);
              }
            } catch (err) {
              console.warn(`    ⚠ Axe error on ${section.id}:`, err.message);
            }
          }
        }
      }

      await context.close();
    }

    console.log(`\nCompletato! ${totalScreenshots} screenshot salvati in ${OUT_DIR}`);
    if (RUN_AXE) {
      console.log(`Axe violazioni totali: ${totalAxeViolations}`);
    }
  } finally {
    await browser.close();
  }
}

main().catch((err) => {
  console.error("Errore durante la cattura degli screenshot:", err);
  process.exit(1);
});
