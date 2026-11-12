"use client";

import { useEffect, useRef } from "react";
import { play, soundForSan } from "@/lib/sounds";

/**
 * Plays a sound when a move is added to the list (not when a game is first loaded or the list is cut back),
 * and an end-of-game sound when `over` turns true.
 */
export function useMoveSounds(sans: string[], over: boolean) {
  const prev = useRef<{ n: number; over: boolean } | null>(null);
  useEffect(() => {
    const p = prev.current;
    prev.current = { n: sans.length, over };
    if (!p) return;
    if (!p.over && over) play("end");
    else if (sans.length === p.n + 1) play(soundForSan(sans[sans.length - 1]));
  }, [sans, over]);
}
