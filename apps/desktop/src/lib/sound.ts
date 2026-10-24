/**
 * Sound kit synthesized with the Web Audio API: every sound is generated at
 * runtime, so the app ships no audio files and no third-party assets.
 *
 * The palette follows the forge theme: metallic clinks for selections, a
 * hammer strike for big moments, air whooshes for panels, soft ticks for hover.
 */
import { getSettings, subscribeSettings } from "./settings";

export type SoundName = "hover" | "select" | "open" | "close" | "page" | "toggle" | "chime" | "error" | "forge" | "knock" | "latch" | "bubble" | "glass";

type SoundGroup = "soundUi" | "soundEvents" | "soundShelf";

/** Which switch in the settings owns each sound. */
const GROUP: Record<SoundName, SoundGroup> = {
  hover: "soundUi",
  select: "soundUi",
  open: "soundUi",
  close: "soundUi",
  page: "soundUi",
  toggle: "soundUi",
  chime: "soundEvents",
  error: "soundEvents",
  forge: "soundEvents",
  bubble: "soundEvents",
  glass: "soundEvents",
  knock: "soundShelf",
  latch: "soundShelf",
};

const UI_SOUNDS: ReadonlySet<SoundName> = new Set(["hover", "page", "toggle"]);

class SoundEngine {
  private ctx: AudioContext | null = null;
  private master: GainNode | null = null;
  private ui: GainNode | null = null;
  private fx: GainNode | null = null;
  private noise: AudioBuffer | null = null;
  private lastHover = 0;

  constructor() {
    subscribeSettings(() => this.applyVolumes());
  }

  private ensure(): AudioContext | null {
    if (typeof AudioContext === "undefined") return null;
    if (!this.ctx) {
      const ctx = new AudioContext();
      const compressor = ctx.createDynamicsCompressor();
      compressor.threshold.value = -18;
      compressor.ratio.value = 4;
      this.master = ctx.createGain();
      this.ui = ctx.createGain();
      this.fx = ctx.createGain();
      this.ui.connect(this.master);
      this.fx.connect(this.master);
      this.master.connect(compressor);
      compressor.connect(ctx.destination);
      const buffer = ctx.createBuffer(1, ctx.sampleRate, ctx.sampleRate);
      const data = buffer.getChannelData(0);
      for (let i = 0; i < data.length; i++) data[i] = Math.random() * 2 - 1;
      this.noise = buffer;
      this.ctx = ctx;
      this.applyVolumes();
    }
    if (this.ctx.state === "suspended") void this.ctx.resume();
    return this.ctx;
  }

  private applyVolumes() {
    if (!this.ctx || !this.master || !this.ui || !this.fx) return;
    const s = getSettings();
    const t = this.ctx.currentTime;
    this.master.gain.setTargetAtTime(s.sound ? s.masterVolume : 0, t, 0.02);
    this.ui.gain.setTargetAtTime(s.uiVolume, t, 0.02);
    this.fx.gain.setTargetAtTime(s.fxVolume, t, 0.02);
  }

  /** A single oscillator with a fast attack and exponential decay. */
  private tone(out: AudioNode, freq: number, start: number, dur: number, gain: number, type: OscillatorType = "sine", freqEnd?: number) {
    const ctx = this.ctx!;
    const osc = ctx.createOscillator();
    const env = ctx.createGain();
    osc.type = type;
    osc.frequency.setValueAtTime(freq, start);
    if (freqEnd) osc.frequency.exponentialRampToValueAtTime(freqEnd, start + dur);
    env.gain.setValueAtTime(0.0001, start);
    env.gain.exponentialRampToValueAtTime(gain, start + 0.004);
    env.gain.exponentialRampToValueAtTime(0.0001, start + dur);
    osc.connect(env).connect(out);
    osc.start(start);
    osc.stop(start + dur + 0.02);
  }

  /** Filtered noise, optionally sweeping the filter frequency. */
  private noiseBurst(out: AudioNode, start: number, dur: number, gain: number, filter: BiquadFilterType, from: number, to = from, q = 0.8) {
    const ctx = this.ctx!;
    const src = ctx.createBufferSource();
    src.buffer = this.noise;
    const biquad = ctx.createBiquadFilter();
    biquad.type = filter;
    biquad.Q.value = q;
    biquad.frequency.setValueAtTime(from, start);
    biquad.frequency.exponentialRampToValueAtTime(to, start + dur);
    const env = ctx.createGain();
    env.gain.setValueAtTime(0.0001, start);
    env.gain.exponentialRampToValueAtTime(gain, start + Math.min(0.05, dur / 3));
    env.gain.exponentialRampToValueAtTime(0.0001, start + dur);
    src.connect(biquad).connect(env).connect(out);
    src.start(start, Math.random() * 0.5);
    src.stop(start + dur + 0.02);
  }

  /** Inharmonic partials of struck metal. */
  private clink(out: AudioNode, start: number, base: number, gain: number) {
    const partials = [1, 2.25, 3.32, 4.52];
    partials.forEach((ratio, i) => this.tone(out, base * ratio, start, 0.42 - i * 0.08, gain / (i + 1.4)));
    this.noiseBurst(out, start, 0.03, gain * 0.4, "highpass", 3500);
  }

  /** `force` is for the "Prova" buttons, which play even when the group is off. */
  play(name: SoundName, force = false) {
    const s = getSettings();
    if (!s.sound || s.masterVolume === 0) return;
    if (!force && !s[GROUP[name]]) return;
    if (name === "hover") {
      const now = performance.now();
      if (now - this.lastHover < 45) return;
      this.lastHover = now;
    }
    const ctx = this.ensure();
    if (!ctx || !this.ui || !this.fx) return;
    const out = UI_SOUNDS.has(name) ? this.ui : this.fx;
    const t = ctx.currentTime + 0.005;
    switch (name) {
      case "hover":
        this.tone(out, 2100, t, 0.035, 0.035, "triangle", 1600);
        break;
      case "toggle":
        this.tone(out, 880, t, 0.06, 0.06, "triangle", 1320);
        break;
      case "page":
        this.noiseBurst(out, t, 0.2, 0.05, "bandpass", 700, 2200, 1.1);
        this.tone(out, 660, t + 0.02, 0.12, 0.025, "sine", 990);
        break;
      case "select":
        this.clink(out, t, 1180 + Math.random() * 90, 0.16);
        break;
      case "open":
        this.noiseBurst(out, t, 0.32, 0.09, "bandpass", 320, 2800, 1.2);
        this.tone(out, 520, t + 0.03, 0.26, 0.03, "sine", 820);
        break;
      case "close":
        this.noiseBurst(out, t, 0.24, 0.06, "bandpass", 2400, 360, 1.2);
        break;
      case "chime":
        this.tone(out, 1318.5, t, 0.55, 0.07);
        this.tone(out, 1975.5, t + 0.07, 0.6, 0.06);
        this.tone(out, 2637, t + 0.14, 0.7, 0.03);
        break;
      case "error":
        this.tone(out, 180, t, 0.22, 0.07, "square", 150);
        this.tone(out, 191, t, 0.22, 0.05, "square", 160);
        break;
      case "knock":
        // A box set down on a wooden shelf: two soft, hollow knocks.
        this.tone(out, 210, t, 0.14, 0.22, "sine", 95);
        this.noiseBurst(out, t, 0.05, 0.12, "lowpass", 900, 300);
        this.tone(out, 190, t + 0.09, 0.1, 0.1, "sine", 90);
        break;
      case "latch":
        // A metal clasp: two dry clicks and a small ring.
        this.noiseBurst(out, t, 0.02, 0.18, "highpass", 3800);
        this.noiseBurst(out, t + 0.05, 0.025, 0.14, "highpass", 2600);
        this.tone(out, 2350, t + 0.05, 0.16, 0.035, "triangle", 2200);
        break;
      case "bubble":
        // The lab: a few small bubbles popping, rising in pitch.
        [0, 0.05, 0.11].forEach((d, i) => this.tone(out, 620 + i * 260, t + d, 0.07, 0.03, "sine", 900 + i * 380));
        break;
      case "glass":
        // A glass vial tapped: bright, short partials.
        this.tone(out, 2793, t, 0.32, 0.05);
        this.tone(out, 4186, t + 0.01, 0.22, 0.025);
        this.tone(out, 5588, t + 0.02, 0.14, 0.012);
        break;
      case "forge":
        // Hammer on anvil: low thud, impact noise, ringing metal.
        this.tone(out, 150, t, 0.3, 0.32, "sine", 52);
        this.noiseBurst(out, t, 0.08, 0.22, "lowpass", 1800, 600);
        this.clink(out, t + 0.01, 980, 0.2);
        this.noiseBurst(out, t + 0.05, 0.6, 0.025, "highpass", 5000, 7000);
        break;
    }
  }
}

export const sound = new SoundEngine();
