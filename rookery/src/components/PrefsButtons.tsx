"use client";

import { usePrefs, type ThemeChoice } from "@/lib/prefs";
import { play } from "@/lib/sounds";

const NEXT: Record<ThemeChoice, ThemeChoice> = { system: "dark", dark: "light", light: "system" };
const LABEL: Record<ThemeChoice, string> = { system: "Theme: follow device", dark: "Theme: dark", light: "Theme: light" };

/** Header buttons: cycle the colour theme, and mute or unmute sounds. */
export function PrefsButtons() {
  const [prefs, update] = usePrefs();
  const btn = "rounded-md p-2 text-dim hover:bg-surface-2 hover:text-ink";
  return (
    <>
      <button type="button" className={btn} onClick={() => update({ theme: NEXT[prefs.theme] })} aria-label={`${LABEL[prefs.theme]}. Press to change.`} title={LABEL[prefs.theme]}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          {prefs.theme === "dark" ? <path d="M21 12.8A9 9 0 1 1 11.2 3a7 7 0 0 0 9.8 9.8z" /> : prefs.theme === "light" ? (
            <><circle cx="12" cy="12" r="4" /><path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" /></>
          ) : (
            <><rect x="3" y="4" width="18" height="12" rx="2" /><path d="M8 20h8M12 16v4" /></>
          )}
        </svg>
      </button>
      <button type="button" className={btn} aria-pressed={prefs.sound} aria-label={prefs.sound ? "Sound on. Press to mute." : "Sound off. Press to turn on."} title={prefs.sound ? "Sound on" : "Sound off"}
        onClick={() => { update({ sound: !prefs.sound }); if (!prefs.sound) setTimeout(() => play("move"), 0); }}>
        <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden>
          <path d="M11 5 6 9H2v6h4l5 4z" />
          {prefs.sound ? <path d="M15.5 8.5a5 5 0 0 1 0 7M19 5a10 10 0 0 1 0 14" /> : <path d="m22 9-6 6M16 9l6 6" />}
        </svg>
      </button>
    </>
  );
}
