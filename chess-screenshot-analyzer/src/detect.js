// Locate the 8x8 board in a screenshot.
//
// Every interior grid line of a checkerboard is a strong colour edge that runs the full
// height (or width) of the board, because neighbouring squares always alternate between
// light and dark. So:
//   1. find 7 equally spaced peaks in the column edge profile, then in the row profile
//      (restricted to the columns found) - this gives the square size and the grid phase,
//      but not *which* 7 of the 9 lines they are,
//   2. try the three possible alignments per axis and keep the one where the 8x8 region
//      really is a two-colour checkerboard,
//   3. fine-align the box using all 9 lines at sub-pixel precision.
//
// Input : { data: Uint8ClampedArray (RGBA), width, height }
// Output: { x, y, size, strength }   strength in [0..1]; below ~0.6 it is probably not a board.

const EDGE_T = 48; // |dR|+|dG|+|dB| needed to count as an edge pixel

function colEdges(img, y0, y1, x0 = 0, x1 = img.width) {
  const { data, width: W, height: H } = img;
  const prof = new Float32Array(W);
  const xa = Math.max(1, Math.floor(x0)), xb = Math.min(W, Math.ceil(x1));
  for (let y = Math.max(0, Math.floor(y0)); y < Math.min(H, Math.ceil(y1)); y++) {
    let i = (y * W + xa) * 4;
    for (let x = xa; x < xb; x++, i += 4) {
      const d = Math.abs(data[i] - data[i - 4]) + Math.abs(data[i + 1] - data[i - 3]) + Math.abs(data[i + 2] - data[i - 2]);
      if (d > EDGE_T) prof[x]++;
    }
  }
  return prof;
}

function rowEdges(img, x0, x1, y0 = 0, y1 = img.height) {
  const { data, width: W, height: H } = img;
  const prof = new Float32Array(H);
  const xa = Math.max(0, Math.floor(x0)), xb = Math.min(W, Math.ceil(x1));
  const up = W * 4;
  for (let y = Math.max(1, Math.floor(y0)); y < Math.min(H, Math.ceil(y1)); y++) {
    let i = (y * W + xa) * 4;
    for (let x = xa; x < xb; x++, i += 4) {
      const d = Math.abs(data[i] - data[i - up]) + Math.abs(data[i + 1] - data[i + 1 - up]) + Math.abs(data[i + 2] - data[i + 2 - up]);
      if (d > EDGE_T) prof[y]++;
    }
  }
  return prof;
}

const peakAt = (prof, p) => {
  const r = Math.round(p);
  if (r < 1 || r >= prof.length - 1) return 0;
  return Math.max(prof[r - 1], prof[r], prof[r + 1]);
};

// Find x1 (first of 7 interior-looking lines) and spacing s whose 7 lines hit profile peaks.
function coarseGrid(prof, sMin, sMax, sStep, lo = 1, hi = prof.length) {
  let best = { score: -Infinity, x1: lo, s: sMin };
  for (let s = sMin; s <= sMax + 1e-9; s += sStep) {
    const top = Math.min(prof.length - 1, Math.floor(hi)) - Math.ceil(6 * s);
    for (let x1 = Math.max(1, Math.floor(lo)); x1 <= top; x1++) {
      let sc = 0;
      for (let k = 0; k < 7; k++) sc += peakAt(prof, x1 + k * s);
      if (sc > best.score) best = { score: sc, x1, s };
    }
  }
  return best;
}

// Refine the position/size of all 9 lines (outer two count half) at sub-pixel precision.
function fineGrid(prof, x0, s, capPos = 0.08, capScale = 0.03) {
  let best = { score: -Infinity, x0, s };
  const dx = Math.max(2, s * capPos);
  for (let ss = s * (1 - capScale); ss <= s * (1 + capScale) + 1e-9; ss += Math.max(0.1, s * 0.002)) {
    for (let xx = x0 - dx; xx <= x0 + dx + 1e-9; xx += 0.25) {
      let sc = 0;
      for (let k = 0; k <= 8; k++) sc += peakAt(prof, xx + k * ss) * (k === 0 || k === 8 ? 0.5 : 1);
      if (sc > best.score) best = { score: sc, x0: xx, s: ss };
    }
  }
  return best;
}

// How checkerboard-like is the 8x8 region? 0..1 (1 = every square matches its parity colour).
export function boardScore(img, x0, y0, s) {
  const { data, width: W, height: H } = img;
  if (x0 < -1 || y0 < -1 || x0 + 8 * s > W + 1 || y0 + 8 * s > H + 1) return 0;
  const p = Math.max(1, Math.round(s * 0.07));
  const inset = Math.max(1, Math.round(s * 0.05));
  const patch = (px, py) => {
    let r = 0, g = 0, b = 0, n = 0;
    for (let y = py; y < py + p; y++) for (let x = px; x < px + p; x++) {
      const xx = Math.min(W - 1, Math.max(0, Math.round(x))), yy = Math.min(H - 1, Math.max(0, Math.round(y)));
      const i = (yy * W + xx) * 4; r += data[i]; g += data[i + 1]; b += data[i + 2]; n++;
    }
    return [r / n, g / n, b / n];
  };
  const med = (a) => { const t = [...a].sort((u, v) => u - v); return t.length % 2 ? t[(t.length - 1) / 2] : (t[t.length / 2 - 1] + t[t.length / 2]) / 2; };
  const cols = [];
  for (let r = 0; r < 8; r++) for (let c = 0; c < 8; c++) {
    const cx = x0 + c * s, cy = y0 + r * s;
    const ps = [patch(cx + inset, cy + inset), patch(cx + s - inset - p, cy + inset), patch(cx + inset, cy + s - inset - p), patch(cx + s - inset - p, cy + s - inset - p)];
    cols.push([0, 1, 2].map((ch) => med(ps.map((q) => q[ch]))));
  }
  const parityMed = [0, 1].map((par) => [0, 1, 2].map((ch) => med(cols.filter((_, i) => ((i >> 3) + (i & 7)) % 2 === par).map((c) => c[ch]))));
  const contrast = parityMed[0].reduce((a, v, ch) => a + Math.abs(v - parityMed[1][ch]), 0);
  if (contrast < 30) return 0;
  let ok = 0;
  cols.forEach((c, i) => {
    const m = parityMed[((i >> 3) + (i & 7)) % 2];
    if (Math.abs(c[0] - m[0]) + Math.abs(c[1] - m[1]) + Math.abs(c[2] - m[2]) <= 60) ok++;
  });
  return ok / 64;
}

export function detectBoard(img) {
  const { width: W, height: H } = img;
  const sMaxAll = Math.min(W, H) / 8;
  const sMinAll = Math.max(12, sMaxAll / 12);

  // 1. Column phase from the whole image, then row phase inside those columns.
  const px = coarseGrid(colEdges(img, 0, H), sMinAll, sMaxAll, 0.5);
  const xa = px.x1 - px.s, xb = px.x1 + 7 * px.s; // generous: covers all three alignments
  const py = coarseGrid(rowEdges(img, xa, xb, 0, H), Math.max(sMinAll, px.s - 4), Math.min(sMaxAll + 4, px.s + 4), 0.25);
  let s = (px.s + py.s) / 2;

  // 2. Pick the alignment (which 7 of the 9 lines we found) that yields a real checkerboard.
  let best = null;
  for (const jx of [1, 0, 2]) {
    for (const jy of [1, 0, 2]) {
      const x0 = px.x1 - jx * px.s, y0 = py.x1 - jy * py.s;
      const sc = boardScore(img, x0, y0, s);
      if (!best || sc > best.sc + 1e-9) best = { sc, x0, y0 };
    }
  }

  // 3. Sub-pixel refinement with all 9 lines, twice (each axis is restricted by the other).
  let { x0, y0 } = best;
  for (let iter = 0; iter < 2; iter++) {
    const fx = fineGrid(colEdges(img, y0, y0 + 8 * s), x0, s);
    x0 = fx.x0;
    const fy = fineGrid(rowEdges(img, x0, x0 + 8 * fx.s), y0, fx.s);
    y0 = fy.x0;
    s = (fx.s + fy.s) / 2;
  }
  return { x: x0, y: y0, size: 8 * s, strength: boardScore(img, x0, y0, s) };
}

/**
 * Snap a roughly placed box (e.g. dragged by hand) onto the real grid lines.
 * Returns the snapped box, or the original if snapping would make it look less like a board.
 */
export function snapBoard(img, box) {
  let x0 = box.x, y0 = box.y, s = box.size / 8;
  const before = boardScore(img, x0, y0, s);
  for (let iter = 0; iter < 3; iter++) {
    const fx = fineGrid(colEdges(img, y0, y0 + 8 * s), x0, s, 0.3, 0.06);
    x0 = fx.x0;
    const fy = fineGrid(rowEdges(img, x0, x0 + 8 * fx.s), y0, fx.s, 0.3, 0.06);
    y0 = fy.x0;
    s = (fx.s + fy.s) / 2;
  }
  const after = boardScore(img, x0, y0, s);
  if (after >= 0.6 && after >= before - 0.02) return { x: x0, y: y0, size: 8 * s, strength: after };
  return { ...box, strength: before };
}
