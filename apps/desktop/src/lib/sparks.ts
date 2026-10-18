/**
 * Forge sparks: a short burst of glowing particles drawn on a full-window
 * canvas above everything. The animation runs only while sparks are alive.
 */
import { getSettings, prefersReducedMotion } from "./settings";

interface Spark {
  x: number;
  y: number;
  px: number;
  py: number;
  vx: number;
  vy: number;
  life: number;
  maxLife: number;
  color: string;
  width: number;
}

const COLORS = ["#fff4cf", "#ffd66b", "#ffab3d", "#ff6a26"];
let canvas: HTMLCanvasElement | null = null;
let ctx: CanvasRenderingContext2D | null = null;
let sparks: Spark[] = [];
let running = false;

function ensureCanvas() {
  if (canvas) return;
  canvas = document.createElement("canvas");
  canvas.setAttribute("aria-hidden", "true");
  Object.assign(canvas.style, { position: "fixed", inset: "0", pointerEvents: "none", zIndex: "9999" });
  document.body.appendChild(canvas);
  ctx = canvas.getContext("2d");
  const resize = () => {
    const dpr = window.devicePixelRatio || 1;
    canvas!.width = innerWidth * dpr;
    canvas!.height = innerHeight * dpr;
    canvas!.style.width = `${innerWidth}px`;
    canvas!.style.height = `${innerHeight}px`;
    ctx!.setTransform(dpr, 0, 0, dpr, 0, 0);
  };
  resize();
  addEventListener("resize", resize);
}

function frame() {
  if (!ctx || !canvas) return;
  ctx.clearRect(0, 0, innerWidth, innerHeight);
  ctx.globalCompositeOperation = "lighter";
  sparks = sparks.filter((s) => s.life > 0);
  for (const s of sparks) {
    s.px = s.x;
    s.py = s.y;
    s.vy += 0.16;
    s.vx *= 0.985;
    s.x += s.vx;
    s.y += s.vy;
    s.life -= 1;
    const alpha = Math.max(0, s.life / s.maxLife);
    ctx.strokeStyle = s.color;
    ctx.globalAlpha = alpha;
    ctx.lineWidth = s.width;
    ctx.lineCap = "round";
    ctx.beginPath();
    ctx.moveTo(s.px, s.py);
    ctx.lineTo(s.x, s.y);
    ctx.stroke();
  }
  ctx.globalAlpha = 1;
  if (sparks.length > 0) {
    requestAnimationFrame(frame);
  } else {
    running = false;
    ctx.clearRect(0, 0, innerWidth, innerHeight);
  }
}

/** Throws sparks from a point (viewport coordinates). */
export function burst(x: number, y: number, count = 22, power = 1) {
  if (!getSettings().cardEffects || prefersReducedMotion()) return;
  ensureCanvas();
  for (let i = 0; i < count; i++) {
    const angle = -Math.PI / 2 + (Math.random() - 0.5) * Math.PI * 1.3;
    const speed = (2.5 + Math.random() * 5) * power;
    const life = 26 + Math.random() * 30;
    sparks.push({
      x,
      y,
      px: x,
      py: y,
      vx: Math.cos(angle) * speed,
      vy: Math.sin(angle) * speed,
      life,
      maxLife: life,
      color: COLORS[Math.floor(Math.random() * COLORS.length)],
      width: 1 + Math.random() * 1.8,
    });
  }
  if (!running) {
    running = true;
    requestAnimationFrame(frame);
  }
}

/** Sparks from the center of an element. */
export function burstFrom(el: Element | null, count?: number, power?: number) {
  if (!el) return;
  const r = el.getBoundingClientRect();
  burst(r.left + r.width / 2, r.top + r.height / 2, count, power);
}
