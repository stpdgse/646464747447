"use client";

import { useEffect, useRef, useState } from "react";
import { copyBoardScreenshot } from "@/lib/screenshot";

export const SCREENSHOT_KEY = "s";

const typing = (t: EventTarget | null) => {
  const el = t as HTMLElement | null;
  return !!el && (el.tagName === "INPUT" || el.tagName === "TEXTAREA" || el.tagName === "SELECT" || el.isContentEditable);
};

/**
 * Press the single key S (no modifiers) anywhere on a game page to copy a picture of the board to the clipboard.
 * Does nothing while you are typing in a chat box or form.
 */
export function ScreenshotKey() {
  const [msg, setMsg] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const flash = (text: string) => {
      setMsg(text);
      if (timer.current) clearTimeout(timer.current);
      timer.current = setTimeout(() => setMsg(null), 2200);
    };
    const onKey = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() !== SCREENSHOT_KEY || e.repeat || e.ctrlKey || e.metaKey || e.altKey || typing(e.target)) return;
      copyBoardScreenshot().then(
        (r) => flash(r === "copied" ? "Board copied to your clipboard" : "Clipboard blocked, saved as a file instead"),
        () => flash("Couldn't capture the board"),
      );
    };
    window.addEventListener("keydown", onKey);
    return () => {
      window.removeEventListener("keydown", onKey);
      if (timer.current) clearTimeout(timer.current);
    };
  }, []);

  return (
    <>
      <p className="text-xs text-dim">Tip: press <kbd className="rounded border border-line bg-surface-2 px-1.5 font-mono">S</kbd> to copy a picture of the board.</p>
      <div role="status" aria-live="polite" className={`pointer-events-none fixed bottom-6 left-1/2 z-50 -translate-x-1/2 rounded-full bg-surface-2 px-4 py-2 text-sm font-semibold shadow-xl transition-opacity ${msg ? "opacity-100" : "opacity-0"}`}>
        {msg}
      </div>
    </>
  );
}
