"use client";

import { readPrefs } from "@/lib/prefs";

/**
 * Sound effects synthesised with the Web Audio API, so there are no audio files to download or license.
 * Browsers only allow audio after a user gesture; until then calls are silently ignored.
 */
export type SoundName = "move" | "capture" | "check" | "castle" | "start" | "end" | "lowtime" | "notify" | "illegal";

let ctx: AudioContext | null = null;
const audio = () => {
  if (typeof window === "undefined") return null;
  if (!ctx) {
    const AC = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
    if (!AC) return null;
    ctx = new AC();
  }
  if (ctx.state === "suspended") void ctx.resume().catch(() => {});
  return ctx;
};

type Tone = { f: number; t?: number; d: number; type?: OscillatorType; g?: number; slideTo?: number };

const RECIPES: Record<SoundName, Tone[]> = {
  move: [{ f: 220, d: 0.07, type: "triangle", g: 0.9, slideTo: 150 }, { f: 900, d: 0.03, type: "square", g: 0.12 }],
  capture: [{ f: 180, d: 0.11, type: "triangle", g: 1, slideTo: 90 }, { f: 1200, d: 0.05, type: "sawtooth", g: 0.18 }],
  castle: [{ f: 220, d: 0.07, type: "triangle", g: 0.8, slideTo: 150 }, { f: 260, t: 0.1, d: 0.07, type: "triangle", g: 0.8, slideTo: 170 }],
  check: [{ f: 660, d: 0.1, type: "square", g: 0.35 }, { f: 880, t: 0.09, d: 0.14, type: "square", g: 0.35 }],
  start: [{ f: 440, d: 0.12, type: "sine", g: 0.6 }, { f: 660, t: 0.12, d: 0.2, type: "sine", g: 0.6 }],
  end: [{ f: 523, d: 0.14, type: "sine", g: 0.6 }, { f: 392, t: 0.14, d: 0.14, type: "sine", g: 0.6 }, { f: 330, t: 0.28, d: 0.3, type: "sine", g: 0.6 }],
  lowtime: [{ f: 1000, d: 0.05, type: "square", g: 0.25 }],
  notify: [{ f: 784, d: 0.1, type: "sine", g: 0.5 }, { f: 1175, t: 0.1, d: 0.16, type: "sine", g: 0.5 }],
  illegal: [{ f: 140, d: 0.16, type: "sawtooth", g: 0.35, slideTo: 100 }],
};

export function play(name: SoundName) {
  const prefs = readPrefs();
  if (!prefs.sound || prefs.volume <= 0) return;
  const c = audio();
  if (!c) return;
  const now = c.currentTime;
  for (const tone of RECIPES[name]) {
    const osc = c.createOscillator();
    const gain = c.createGain();
    const start = now + (tone.t ?? 0);
    const peak = (tone.g ?? 0.6) * prefs.volume * 0.5;
    osc.type = tone.type ?? "sine";
    osc.frequency.setValueAtTime(tone.f, start);
    if (tone.slideTo) osc.frequency.exponentialRampToValueAtTime(tone.slideTo, start + tone.d);
    gain.gain.setValueAtTime(0.0001, start);
    gain.gain.exponentialRampToValueAtTime(Math.max(peak, 0.0002), start + 0.008);
    gain.gain.exponentialRampToValueAtTime(0.0001, start + tone.d);
    osc.connect(gain).connect(c.destination);
    osc.start(start);
    osc.stop(start + tone.d + 0.02);
  }
}

/** Which sound a SAN move deserves. */
export const soundForSan = (san: string): SoundName =>
  /[+#]$/.test(san) ? "check" : san.startsWith("O-O") ? "castle" : san.includes("x") ? "capture" : "move";
