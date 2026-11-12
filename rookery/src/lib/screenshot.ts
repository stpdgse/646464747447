"use client";

/**
 * Draws the board on the page into a PNG, exactly as it looks right now (theme colours, piece set, orientation,
 * last-move and check highlights, coordinates). Pieces are read from the live DOM, so whatever the player sees is what they get.
 */

const imageCache = new Map<string, Promise<HTMLImageElement>>();
function loadImage(url: string): Promise<HTMLImageElement> {
  let p = imageCache.get(url);
  if (!p) {
    p = new Promise((resolve, reject) => {
      const img = new Image();
      img.onload = () => resolve(img);
      img.onerror = () => reject(new Error(`Could not load ${url}`));
      img.src = url;
    });
    imageCache.set(url, p);
  }
  return p;
}

const urlOf = (bg: string): string | null => /url\(["']?(.*?)["']?\)/.exec(bg)?.[1] ?? null;

export function findBoard(root: ParentNode = document): HTMLElement | null {
  for (const el of root.querySelectorAll<HTMLElement>(".rk-board")) {
    const r = el.getBoundingClientRect();
    if (r.width > 0 && r.height > 0 && el.querySelector("cg-board")) return el;
  }
  return null;
}

export async function renderBoardPng(host: HTMLElement): Promise<Blob> {
  const board = host.querySelector<HTMLElement>("cg-board");
  if (!board) throw new Error("No board on this page.");
  const rect = board.getBoundingClientRect();
  const css = getComputedStyle(host);
  const light = css.getPropertyValue("--sq-light").trim() || "#eeeed2";
  const dark = css.getPropertyValue("--sq-dark").trim() || "#769656";

  const target = 960; // sharp but small enough to paste anywhere
  const size = Math.min(1600, Math.max(target, Math.round(rect.width)));
  const sq = size / 8;
  const k = size / rect.width;
  const canvas = document.createElement("canvas");
  canvas.width = canvas.height = size;
  const ctx = canvas.getContext("2d");
  if (!ctx) throw new Error("Canvas is not available.");

  const black = host.querySelector(".cg-wrap")?.classList.contains("orientation-black") ?? false;

  // squares
  for (let r = 0; r < 8; r++) {
    for (let f = 0; f < 8; f++) {
      ctx.fillStyle = (r + f) % 2 === 0 ? light : dark;
      ctx.fillRect(f * sq, r * sq, sq, sq);
    }
  }

  // highlights (last move, check, selection)
  for (const el of board.querySelectorAll<HTMLElement>("square")) {
    const cls = el.classList;
    if (!(cls.contains("last-move") || cls.contains("check") || cls.contains("selected"))) continue;
    const b = el.getBoundingClientRect();
    const x = (b.left - rect.left) * k, y = (b.top - rect.top) * k, w = b.width * k, h = b.height * k;
    if (cls.contains("check")) {
      const g = ctx.createRadialGradient(x + w / 2, y + h / 2, 0, x + w / 2, y + h / 2, w * 0.7);
      g.addColorStop(0, "#ff0000"); g.addColorStop(0.25, "#e70000"); g.addColorStop(0.89, "rgba(169,0,0,0)");
      ctx.fillStyle = g;
    } else {
      ctx.fillStyle = cls.contains("selected") ? "rgba(20,85,30,0.5)" : "rgba(155,199,0,0.41)";
    }
    ctx.fillRect(x, y, w, h);
  }

  // coordinates (files along the bottom, ranks down the right edge), coloured as the opposite square
  if (host.querySelector("coords")) {
    ctx.font = `700 ${Math.round(sq * 0.17)}px system-ui, sans-serif`;
    for (let i = 0; i < 8; i++) {
      const file = "abcdefgh"[black ? 7 - i : i];
      const rank = String(black ? i + 1 : 8 - i);
      ctx.textBaseline = "alphabetic";
      ctx.textAlign = "left";
      ctx.fillStyle = (7 + i) % 2 === 0 ? dark : light;
      ctx.fillText(file, i * sq + sq * 0.06, size - sq * 0.05);
      ctx.textAlign = "right";
      ctx.textBaseline = "top";
      ctx.fillStyle = (i + 7) % 2 === 0 ? dark : light;
      ctx.fillText(rank, size - sq * 0.06, i * sq + sq * 0.05);
    }
  }

  // pieces, where they are right now (including one mid-drag)
  const pieces = [...board.querySelectorAll<HTMLElement>("piece")].filter((p) => !p.classList.contains("ghost"));
  const imgs = await Promise.all(pieces.map((p) => { const u = urlOf(getComputedStyle(p).backgroundImage); return u ? loadImage(u) : Promise.resolve(null); }));
  pieces.forEach((p, i) => {
    const img = imgs[i];
    if (!img) return;
    const b = p.getBoundingClientRect();
    ctx.drawImage(img, (b.left - rect.left) * k, (b.top - rect.top) * k, b.width * k, b.height * k);
  });

  return new Promise((resolve, reject) => canvas.toBlob((blob) => (blob ? resolve(blob) : reject(new Error("Could not make the image."))), "image/png"));
}

export type CopyResult = "copied" | "downloaded";

/** Put a screenshot of the board on the clipboard; if the browser refuses, save it as a file instead. */
export async function copyBoardScreenshot(): Promise<CopyResult> {
  const host = findBoard();
  if (!host) throw new Error("No board to capture.");
  const png = renderBoardPng(host);
  try {
    if (!navigator.clipboard || typeof ClipboardItem === "undefined") throw new Error("no clipboard");
    // Passing the promise keeps the user-gesture permission alive while the image renders (needed by Safari).
    await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
    return "copied";
  } catch {
    const blob = await png;
    const a = document.createElement("a");
    a.href = URL.createObjectURL(blob);
    a.download = "rookery-board.png";
    a.click();
    setTimeout(() => URL.revokeObjectURL(a.href), 4000);
    return "downloaded";
  }
}
