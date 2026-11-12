"use client";

import { useCallback, useSyncExternalStore } from "react";
import { DEFAULT_SETTINGS, normalizeSettings, type BoardSettings } from "@/lib/chess/themes";

const KEY = "rookery.board";
const listeners = new Set<() => void>();
let cachedRaw: string | null | undefined;
let cached: BoardSettings = DEFAULT_SETTINGS;

function snapshot(): BoardSettings {
  let raw: string | null = null;
  try {
    raw = window.localStorage.getItem(KEY);
  } catch {
    /* storage blocked (private mode): fall back to defaults */
  }
  if (raw !== cachedRaw) {
    cachedRaw = raw;
    try {
      cached = raw ? normalizeSettings(JSON.parse(raw)) : DEFAULT_SETTINGS;
    } catch {
      cached = DEFAULT_SETTINGS;
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

/** Board look-and-feel, persisted in localStorage (and shared between every board on the page). */
export function useBoardSettings(): [BoardSettings, (patch: Partial<BoardSettings>) => void] {
  const settings = useSyncExternalStore(subscribe, snapshot, () => DEFAULT_SETTINGS);
  const update = useCallback((patch: Partial<BoardSettings>) => {
    const next = normalizeSettings({ ...snapshot(), ...patch });
    try {
      window.localStorage.setItem(KEY, JSON.stringify(next));
    } catch {
      /* ignore */
    }
    cachedRaw = undefined; // force re-read
    for (const l of listeners) l();
  }, []);
  return [settings, update];
}
