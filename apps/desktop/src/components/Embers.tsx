import { useEffect, useRef } from "react";
import { prefersReducedMotion, useSettings } from "../lib/settings";

interface Ember {
  x: number;
  y: number;
  r: number;
  speed: number;
  sway: number;
  phase: number;
  flicker: number;
}

/**
 * Ambient embers rising slowly behind the content, tinted with the section
 * accent. Capped at ~30 fps and paused when the window is hidden.
 */
export function Embers({ color }: { color: string }) {
  const ref = useRef<HTMLCanvasElement>(null);
  const { ambient } = useSettings();
  const colorRef = useRef(color);
  colorRef.current = color;

  useEffect(() => {
    const canvas = ref.current;
    if (!canvas || !ambient || prefersReducedMotion()) return;
    const ctx = canvas.getContext("2d");
    if (!ctx) return;
    let width = 0;
    let height = 0;
    const resize = () => {
      const dpr = Math.min(window.devicePixelRatio || 1, 2);
      width = canvas.clientWidth;
      height = canvas.clientHeight;
      canvas.width = width * dpr;
      canvas.height = height * dpr;
      ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    };
    resize();
    const observer = new ResizeObserver(resize);
    observer.observe(canvas);
    const spawn = (anywhere: boolean): Ember => ({
      x: Math.random() * width,
      y: anywhere ? Math.random() * height : height + 10,
      r: 0.6 + Math.random() * 1.8,
      speed: 0.15 + Math.random() * 0.45,
      sway: 6 + Math.random() * 18,
      phase: Math.random() * Math.PI * 2,
      flicker: Math.random() * Math.PI * 2,
    });
    const embers = Array.from({ length: 38 }, () => spawn(true));
    let raf = 0;
    let last = 0;
    const draw = (now: number) => {
      raf = requestAnimationFrame(draw);
      if (document.hidden || now - last < 33) return;
      last = now;
      ctx.clearRect(0, 0, width, height);
      ctx.globalCompositeOperation = "lighter";
      for (let i = 0; i < embers.length; i++) {
        const e = embers[i];
        e.y -= e.speed;
        e.phase += 0.012;
        e.flicker += 0.09;
        if (e.y < -10) embers[i] = spawn(false);
        const x = e.x + Math.sin(e.phase) * e.sway;
        const fade = Math.min(1, e.y / (height * 0.5));
        const alpha = (0.35 + Math.sin(e.flicker) * 0.2) * fade;
        const glow = ctx.createRadialGradient(x, e.y, 0, x, e.y, e.r * 5);
        glow.addColorStop(0, colorRef.current);
        glow.addColorStop(1, "transparent");
        ctx.globalAlpha = Math.max(0, alpha);
        ctx.fillStyle = glow;
        ctx.beginPath();
        ctx.arc(x, e.y, e.r * 5, 0, Math.PI * 2);
        ctx.fill();
      }
      ctx.globalAlpha = 1;
    };
    raf = requestAnimationFrame(draw);
    return () => {
      cancelAnimationFrame(raf);
      observer.disconnect();
      ctx.clearRect(0, 0, width, height);
    };
  }, [ambient]);

  return <canvas ref={ref} aria-hidden className="embers pointer-events-none absolute inset-0 h-full w-full" />;
}
