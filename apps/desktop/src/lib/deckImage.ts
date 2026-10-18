/**
 * Draws a deck as a PNG to share: title, format and legality, then Main,
 * Extra and Side Deck one card per copy. Card images come through
 * `card_image_data` as bytes, so the canvas stays exportable.
 */
import { invoke } from "@tauri-apps/api/core";
import type { Card, Deck } from "./api";
import { copyLimit, craftCost, FORMAT_LABELS } from "./deck";

const WIDTH = 1552;
const PAD = 40;
const INNER = WIDTH - PAD * 2;
const RATIO = 614 / 421;

const COLORS = {
  bg: "#120d0a",
  bg2: "#1c1410",
  text: "#f3e6d0",
  muted: "#b59f86",
  faint: "#7d6a58",
  accent: "#ff8a3d",
  border: "rgba(255, 190, 140, 0.16)",
  ok: "#5fcf8f",
  danger: "#ff5a52",
};

const GEMS: [keyof ReturnType<typeof craftCost>, string, string][] = [
  ["ur", "UR", "#e8b3ff"],
  ["sr", "SR", "#f2cf63"],
  ["r", "R", "#7cc0f0"],
  ["n", "N", "#c9c4bd"],
];

async function loadImage(id: number): Promise<ImageBitmap | null> {
  try {
    const data = await invoke<ArrayBuffer>("card_image_data", { id, size: "small" });
    return await createImageBitmap(new Blob([data], { type: "image/jpeg" }));
  } catch {
    return null;
  }
}

function roundRect(ctx: CanvasRenderingContext2D, x: number, y: number, w: number, h: number, r: number) {
  ctx.beginPath();
  if (typeof ctx.roundRect === "function") ctx.roundRect(x, y, w, h, r);
  else ctx.rect(x, y, w, h);
}

interface Grid {
  title: string;
  ids: number[];
  columns: number;
  gap: number;
}

function gridSize(g: Grid) {
  const card = (INNER - g.gap * (g.columns - 1)) / g.columns;
  const rows = Math.max(1, Math.ceil(g.ids.length / g.columns));
  return { card, height: card * RATIO, rows, total: 44 + rows * card * RATIO + (rows - 1) * g.gap };
}

export interface DeckImageOptions {
  side: boolean;
  legal: boolean | null;
}

export async function renderDeckImage(deck: Deck, getCard: (id: number) => Card | undefined, opts: DeckImageOptions): Promise<Blob> {
  await Promise.all([document.fonts.load('700 44px "Cinzel"'), document.fonts.load('600 18px "Inter Variable"')]).catch(() => {});
  const grids: Grid[] = [
    { title: "Main Deck", ids: deck.main, columns: 10, gap: 8 },
    { title: "Extra Deck", ids: deck.extra, columns: 15, gap: 6 },
  ];
  if (opts.side && deck.side.length) grids.push({ title: "Side Deck", ids: deck.side, columns: 15, gap: 6 });

  const ids = [...new Set(grids.flatMap((g) => g.ids))];
  const images = new Map(await Promise.all(ids.map(async (id) => [id, await loadImage(id)] as const)));

  const header = 120;
  const footer = 56;
  const height = PAD + header + grids.reduce((sum, g) => sum + gridSize(g).total + 28, 0) + footer;
  const canvas = document.createElement("canvas");
  canvas.width = WIDTH;
  canvas.height = Math.round(height);
  const ctx = canvas.getContext("2d")!;

  // Forge background: dark leather with an ember glow.
  const bg = ctx.createLinearGradient(0, 0, 0, height);
  bg.addColorStop(0, COLORS.bg2);
  bg.addColorStop(1, COLORS.bg);
  ctx.fillStyle = bg;
  ctx.fillRect(0, 0, WIDTH, height);
  const glow = ctx.createRadialGradient(WIDTH * 0.85, 0, 0, WIDTH * 0.85, 0, WIDTH * 0.6);
  glow.addColorStop(0, "rgba(255, 120, 40, 0.22)");
  glow.addColorStop(1, "rgba(255, 120, 40, 0)");
  ctx.fillStyle = glow;
  ctx.fillRect(0, 0, WIDTH, height);
  ctx.strokeStyle = COLORS.border;
  ctx.lineWidth = 2;
  roundRect(ctx, 12, 12, WIDTH - 24, height - 24, 18);
  ctx.stroke();

  // Title and details.
  ctx.textBaseline = "alphabetic";
  ctx.fillStyle = COLORS.text;
  ctx.font = '700 44px "Cinzel", serif';
  ctx.fillText(deck.name, PAD, PAD + 46, INNER - 420);
  ctx.font = '600 18px "Inter Variable", sans-serif';
  const details = [FORMAT_LABELS[deck.format], `${deck.main.length} / ${deck.extra.length} / ${deck.side.length}`];
  let x = PAD;
  const y = PAD + 86;
  for (const part of details) {
    ctx.fillStyle = COLORS.muted;
    ctx.fillText(part, x, y);
    x += ctx.measureText(part).width + 12;
    ctx.fillStyle = COLORS.faint;
    ctx.fillText("·", x, y);
    x += 20;
  }
  if (opts.legal != null) {
    ctx.fillStyle = opts.legal ? COLORS.ok : COLORS.danger;
    ctx.fillText(opts.legal ? "✓ Legale" : "✕ Non legale", x, y);
  }

  // Master Duel crafting cost, right-aligned.
  if (deck.format === "md") {
    const cost = craftCost(deck, getCard);
    let gx = WIDTH - PAD;
    ctx.font = '700 18px "Inter Variable", sans-serif';
    for (const [key, label, color] of [...GEMS].reverse()) {
      const value = String(cost[key]);
      const vw = ctx.measureText(value).width;
      gx -= vw;
      ctx.fillStyle = COLORS.text;
      ctx.fillText(value, gx, PAD + 44);
      ctx.font = '800 13px "Inter Variable", sans-serif';
      const lw = ctx.measureText(label).width + 12;
      gx -= lw + 8;
      ctx.fillStyle = color;
      roundRect(ctx, gx, PAD + 26, lw, 22, 5);
      ctx.fill();
      ctx.fillStyle = "#120c08";
      ctx.fillText(label, gx + 6, PAD + 42);
      ctx.font = '700 18px "Inter Variable", sans-serif';
      gx -= 22;
    }
  }

  // Sections.
  let top = PAD + header;
  for (const g of grids) {
    const size = gridSize(g);
    ctx.font = '700 22px "Cinzel", serif';
    ctx.fillStyle = COLORS.text;
    const title = g.title.toUpperCase();
    ctx.fillText(title, PAD, top + 24);
    const tw = ctx.measureText(title).width;
    ctx.font = '700 20px "Inter Variable", sans-serif';
    ctx.fillStyle = COLORS.accent;
    ctx.fillText(String(g.ids.length), PAD + tw + 14, top + 24);
    ctx.strokeStyle = COLORS.border;
    ctx.beginPath();
    ctx.moveTo(PAD, top + 36);
    ctx.lineTo(WIDTH - PAD, top + 36);
    ctx.stroke();

    g.ids.forEach((id, i) => {
      const cx = PAD + (i % g.columns) * (size.card + g.gap);
      const cy = top + 44 + Math.floor(i / g.columns) * (size.height + g.gap);
      ctx.save();
      ctx.shadowColor = "rgba(0, 0, 0, 0.55)";
      ctx.shadowBlur = 10;
      ctx.shadowOffsetY = 3;
      roundRect(ctx, cx, cy, size.card, size.height, 5);
      ctx.fillStyle = "#2a201a";
      ctx.fill();
      ctx.restore();
      const image = images.get(id);
      if (image) {
        ctx.save();
        roundRect(ctx, cx, cy, size.card, size.height, 5);
        ctx.clip();
        ctx.drawImage(image, cx, cy, size.card, size.height);
        ctx.restore();
      }
      const card = getCard(id);
      const limit = card ? copyLimit(card, deck.format) : 3;
      if (limit < 3) {
        const r = Math.max(9, size.card * 0.09);
        ctx.fillStyle = limit === 0 ? "#d93b3b" : "#e08a1e";
        ctx.beginPath();
        ctx.arc(cx + r + 3, cy + r + 3, r, 0, Math.PI * 2);
        ctx.fill();
        ctx.fillStyle = "#fff";
        ctx.font = `800 ${Math.round(r * 1.2)}px "Inter Variable", sans-serif`;
        ctx.textAlign = "center";
        ctx.fillText(limit === 0 ? "✕" : String(limit), cx + r + 3, cy + r + 3 + r * 0.42);
        ctx.textAlign = "left";
      }
    });
    top += size.total + 28;
  }

  // Footer.
  ctx.font = '500 15px "Inter Variable", sans-serif';
  ctx.fillStyle = COLORS.faint;
  ctx.fillText("Creato con DuelForge", PAD, height - PAD + 4);
  const date = new Date().toLocaleDateString("it-IT", { day: "numeric", month: "long", year: "numeric" });
  ctx.textAlign = "right";
  ctx.fillText(date, WIDTH - PAD, height - PAD + 4);
  ctx.textAlign = "left";

  return new Promise((resolve, reject) => canvas.toBlob((b) => (b ? resolve(b) : reject(new Error("immagine non creata"))), "image/png"));
}

/** Saves the PNG in Downloads/DuelForge; returns the file path. */
export async function saveDeckImage(blob: Blob, name: string): Promise<string> {
  const bytes = new Uint8Array(await blob.arrayBuffer());
  return invoke<string>("save_deck_image", bytes, { headers: { "x-file-name": encodeURIComponent(name) } });
}
