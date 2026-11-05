// Synthetic "Chess.com screenshot" generator used by the tests.
import fs from 'node:fs';
import path from 'node:path';
import { PNG } from 'pngjs';
import { PIECE_CODES, prepareTemplate } from '../src/recognize.js';

const PIECES_DIR = path.resolve(import.meta.dirname, '../assets/pieces-src');

export function readPng(file) {
  const p = PNG.sync.read(fs.readFileSync(file));
  return { data: new Uint8ClampedArray(p.data), width: p.width, height: p.height };
}

export const themeNames = () => fs.readdirSync(PIECES_DIR).filter((d) => fs.statSync(path.join(PIECES_DIR, d)).isDirectory());

export function loadRawTheme(theme) {
  const out = {};
  for (const c of PIECE_CODES) out[c] = readPng(path.join(PIECES_DIR, theme, c + '.png'));
  return out;
}

export function loadTemplates(themes = themeNames()) {
  const t = {};
  for (const th of themes) {
    t[th] = {};
    const raw = loadRawTheme(th);
    for (const c of PIECE_CODES) t[th][c] = prepareTemplate(raw[c]);
  }
  return t;
}

export const BOARDS = {
  green: { light: [238, 238, 210], dark: [118, 150, 86], hiL: [246, 246, 130], hiD: [185, 202, 67] },
  brown: { light: [240, 217, 181], dark: [181, 136, 99], hiL: [205, 210, 106], hiD: [170, 162, 58] },
  blue: { light: [222, 227, 230], dark: [140, 162, 173], hiL: [200, 215, 140], hiD: [130, 160, 120] },
  purple: { light: [239, 239, 239], dark: [125, 135, 150], hiL: [235, 235, 140], hiD: [150, 165, 90] },
};

// Scale RGBA image to size x size (area average, premultiplied) -> Float arrays.
function scalePiece(img, size) {
  const out = new Float32Array(size * size * 4);
  const k = img.width / size;
  for (let y = 0; y < size; y++) for (let x = 0; x < size; x++) {
    let r = 0, g = 0, b = 0, a = 0, n = 0;
    for (let sy = Math.floor(y * k); sy < Math.max(Math.floor(y * k) + 1, Math.floor((y + 1) * k)); sy++)
      for (let sx = Math.floor(x * k); sx < Math.max(Math.floor(x * k) + 1, Math.floor((x + 1) * k)); sx++) {
        const i = (Math.min(sy, img.height - 1) * img.width + Math.min(sx, img.width - 1)) * 4;
        const al = img.data[i + 3] / 255;
        r += img.data[i] * al; g += img.data[i + 1] * al; b += img.data[i + 2] * al; a += al; n++;
      }
    const o = (y * size + x) * 4;
    out[o] = r / n; out[o + 1] = g / n; out[o + 2] = b / n; out[o + 3] = a / n;
  }
  return out;
}

function rng(seed) { let s = seed >>> 0; return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 2 ** 32); }

/**
 * grid: 8x8 of piece codes|null, row 0 = top row *as drawn*.
 * opts: { theme, board, size, W, H, bx, by, highlight:[[r,c],[r,c]], seed, noise, labels }
 */
export function renderScreenshot(grid, o) {
  const rand = rng(o.seed ?? 1);
  const W = o.W, H = o.H;
  const png = new Uint8ClampedArray(W * H * 4);
  // page background + UI-ish clutter
  for (let i = 0; i < W * H; i++) { png[i * 4] = 38; png[i * 4 + 1] = 36; png[i * 4 + 2] = 33; png[i * 4 + 3] = 255; }
  const rect = (x, y, w, h, c) => {
    for (let yy = Math.max(0, Math.round(y)); yy < Math.min(H, Math.round(y + h)); yy++)
      for (let xx = Math.max(0, Math.round(x)); xx < Math.min(W, Math.round(x + w)); xx++) {
        const i = (yy * W + xx) * 4; png[i] = c[0]; png[i + 1] = c[1]; png[i + 2] = c[2]; png[i + 3] = 255;
      }
  };
  if (o.clutter !== false) {
    for (let k = 0; k < 160; k++) rect(rand() * W, rand() * H, 6 + rand() * 40, 3 + rand() * 10, [70 + rand() * 150, 70 + rand() * 150, 70 + rand() * 150]);
    rect(0, 0, W, o.by * 0.5, [48, 46, 43]);
    rect(0, o.by + o.size + 4, W, 40, [28, 26, 24]);
  }
  const cs = o.size / 8;
  const B = BOARDS[o.board];
  const hl = new Set((o.highlight || []).map(([r, c]) => r * 8 + c));
  const raw = loadRawTheme(o.theme);
  const scaled = {};
  const csi = Math.round(cs);
  for (const c of PIECE_CODES) scaled[c] = scalePiece(raw[c], csi);
  for (let r = 0; r < 8; r++) for (let c = 0; c < 8; c++) {
    const light = (r + c) % 2 === 0;
    const bg = hl.has(r * 8 + c) ? (light ? B.hiL : B.hiD) : (light ? B.light : B.dark);
    const x0 = Math.round(o.bx + c * cs), y0 = Math.round(o.by + r * cs);
    const x1 = Math.round(o.bx + (c + 1) * cs), y1 = Math.round(o.by + (r + 1) * cs);
    rect(x0, y0, x1 - x0, y1 - y0, bg);
    const p = grid[r][c];
    if (p) {
      const s = scaled[p];
      for (let y = 0; y < csi; y++) for (let x = 0; x < csi; x++) {
        const xx = x0 + x, yy = y0 + y; if (xx >= W || yy >= H) continue;
        const a = s[(y * csi + x) * 4 + 3]; const i = (yy * W + xx) * 4;
        for (let ch = 0; ch < 3; ch++) png[i + ch] = s[(y * csi + x) * 4 + ch] + (1 - a) * png[i + ch];
      }
    }
    if (o.labels !== false) { // tiny coordinate marks like Chess.com's
      const lc = light ? B.dark : B.light;
      if (c === 0) rect(x0 + 2, y0 + 2, Math.max(2, cs * 0.08), Math.max(3, cs * 0.12), lc);
      if (r === 7) rect(x1 - cs * 0.15, y1 - cs * 0.17, Math.max(2, cs * 0.08), Math.max(3, cs * 0.12), lc);
    }
  }
  const n = o.noise ?? 2;
  if (n) for (let i = 0; i < W * H * 4; i += 4) { const d = (rand() - 0.5) * 2 * n; png[i] += d; png[i + 1] += d; png[i + 2] += d; }
  return { data: png, width: W, height: H };
}
