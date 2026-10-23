#!/usr/bin/env node
/* global process, console, document, fetch, Buffer, Image */

import fs from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const outDir = path.resolve(__dirname, "../src/assets/textures");

const TEXTURES = [
  {
    name: "leather.webp",
    url: "https://f003.backblazeb2.com/file/ambientCG-Web/media/surface-preview/Leather037/Leather037_SQ_Color.jpg",
  },
  {
    name: "metal.webp",
    url: "https://f003.backblazeb2.com/file/ambientCG-Web/media/surface-preview/Metal009/Metal009_SQ_Color.jpg",
  },
  {
    name: "wood.webp",
    url: "https://f003.backblazeb2.com/file/ambientCG-Web/media/surface-preview/Wood049/Wood049_SQ_Color.jpg",
  },
  {
    name: "matte.webp",
    url: "https://f003.backblazeb2.com/file/ambientCG-Web/media/surface-preview/Plastic010/Plastic010_SQ_Color.jpg",
  },
  {
    name: "cardboard.webp",
    url: "https://f003.backblazeb2.com/file/ambientCG-Web/media/surface-preview/Cardboard001/Cardboard001_SQ_Color.jpg",
  },
];

async function main() {
  await fs.mkdir(outDir, { recursive: true });
  console.log("Launching headless Chrome...");
  const browser = await chromium.launch({
    executablePath: "C:\\Program Files\\Google\\Chrome\\Application\\chrome.exe",
    headless: true,
  });
  const page = await browser.newPage();

  for (const item of TEXTURES) {
    console.log(`Fetching ${item.name} from ${item.url}...`);
    const resp = await fetch(item.url);
    if (!resp.ok) {
      throw new Error(`Failed to fetch ${item.url}: ${resp.status} ${resp.statusText}`);
    }
    const arrayBuffer = await resp.arrayBuffer();
    const base64Jpg = Buffer.from(arrayBuffer).toString("base64");
    const dataUri = `data:image/jpeg;base64,${base64Jpg}`;

    // Render into 512x512 canvas and export as WebP
    const webpDataUri = await page.evaluate(async (src) => {
      const img = new Image();
      await new Promise((resolve, reject) => {
        img.onload = resolve;
        img.onerror = reject;
        img.src = src;
      });
      const canvas = document.createElement("canvas");
      canvas.width = 512;
      canvas.height = 512;
      const ctx = canvas.getContext("2d");
      ctx.drawImage(img, 0, 0, 512, 512);
      return canvas.toDataURL("image/webp", 0.92);
    }, dataUri);

    const base64Webp = webpDataUri.replace(/^data:image\/webp;base64,/, "");
    const buffer = Buffer.from(base64Webp, "base64");
    const dest = path.join(outDir, item.name);
    await fs.writeFile(dest, buffer);
    console.log(`Saved ${item.name}: ${buffer.length} bytes (~${(buffer.length / 1024).toFixed(1)} KB)`);
  }

  await browser.close();
  console.log("All textures saved successfully!");
}

main().catch((err) => {
  console.error("Error processing textures:", err);
  process.exit(1);
});
