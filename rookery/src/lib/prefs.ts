"use client";

import { useCallback, useSyncExternalStore } from "react";

export type ThemeChoice = "system" | "dark" | "light";
export type Prefs = { theme: ThemeChoice; sound: boolean; volume: number; feedback: boolean };

export const DEFAULT_PREFS: Prefs = { theme: "system", sound: true, volume: 0.6, feedback: true };
const KEY = "rookery.prefs";

export function normalizePrefs(raw: unknown): Prefs {
  const r = (raw && typeof raw === "object" ? raw : {}) as Record<string, unknown>;
  return {
    theme: r.theme === "dark" || r.theme === "light" || r.theme === "system" ? r.theme : DEFAULT_PREFS.theme,
    sound: typeof r.sound === "boolean" ? r.sound : DEFAULT_PREFS.sound,
    feedback: typeof r.feedback === "boolean" ? r.feedback : DEFAULT_PREFS.feedback,
    volume: typeof r.volume === "number" && r.volume >= 0 && r.volume <= 1 ? r.volume : DEFAULT_PREFS.volume,
  };
}

/** The theme actually shown, given the choice and the OS setting. */
export const resolveTheme = (choice: ThemeChoice, osDark: boolean): "dark" | "light" => (choice === "system" ? (osDark ? "dark" : "light") : choice);

export function applyTheme(choice: ThemeChoice) {
  const dark = window.matchMedia("(prefers-color-scheme: dark)").matches;
  const t = resolveTheme(choice, dark);
  document.documentElement.dataset.theme = t;
  document.querySelector('meta[name="theme-color"]')?.setAttribute("content", t === "dark" ? "#161512" : "#f3f1ed");
}

const listeners = new Set<() => void>();
let cachedRaw: string | null | undefined;
let cached: Prefs = DEFAULT_PREFS;

export function readPrefs(): Prefs {
  let raw: string | null = null;
  try {
    raw = window.localStorage.getItem(KEY);
  } catch {
    /* storage blocked */
  }
  if (raw !== cachedRaw) {
    cachedRaw = raw;
    try {
      cached = raw ? normalizePrefs(JSON.parse(raw)) : DEFAULT_PREFS;
    } catch {
      cached = DEFAULT_PREFS;
    }
  }
  return cached;
}

function subscribe(cb: () => void) {
  listeners.add(cb);
  window.addEventListener("storage", cb);
  return () => {
    listeners.delete(cb);
    window.removeEventListener("storage", cb);
  };
}

/** Site-wide preferences (theme, sound), kept in localStorage and shared by every component. */
export function usePrefs(): [Prefs, (patch: Partial<Prefs>) => void] {
  const prefs = useSyncExternalStore(subscribe, readPrefs, () => DEFAULT_PREFS);
  const update = useCallback((patch: Partial<Prefs>) => {
    const next = normalizePrefs({ ...readPrefs(), ...patch });
    try {
      window.localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      /* ignore */
    }
    cachedRaw = undefined;
    if (patch.theme) applyTheme(next.theme);
    for (const l of listeners) l();
  }, []);
  return [prefs, update];
}

/** Runs before first paint so the page never flashes the wrong theme. Kept as a string for an inline <script>. */
export const THEME_BOOT_SCRIPT = `(function(){try{var p=JSON.parse(localStorage.getItem("${KEY}")||"{}");var c=p.theme==="dark"||p.theme==="light"?p.theme:"system";var d=window.matchMedia("(prefers-color-scheme: dark)").matches;document.documentElement.dataset.theme=c==="system"?(d?"dark":"light"):c;}catch(e){}})();`;
