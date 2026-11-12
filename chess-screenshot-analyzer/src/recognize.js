// Identify the piece on each square by template matching against Chess.com's piece sets.
//
// For every square we estimate its background colour from the four corners (so light/dark
// squares, move highlights and textured boards all work), composite each candidate piece
// image over that background, and pick the candidate with the smallest squared colour
// error. An "empty" candidate (just the flat background) competes too. We try every
// bundled theme, keep the one that explains the whole board best, then redo the match with
// small shifts so a slightly misaligned crop still lines up.

export const PIECE_CODES = ['wp', 'wn', 'wb', 'wr', 'wq', 'wk', 'bp', 'bn', 'bb', 'br', 'bq', 'bk'];

export const R = 32; // matching resolution per square

// Area-average a (fractional) rectangle of an RGBA image down to R x R.
// premult=true returns premultiplied RGB plus alpha; otherwise plain RGB.
function resample(img, x, y, w, h, premult) {
  const { data, width: W, height: H } = img;
  const nx = Math.min(6, Math.max(1, Math.ceil(w / R)));
  const ny = Math.min(6, Math.max(1, Math.ceil(h / R)));
  const rgb = new Float32Array(R * R * 3);
  const alpha = premult ? new Float32Array(R * R) : null;
  const sw = w / (R * nx);
  const sh = h / (R * ny);
  for (let j = 0; j < R; j++) {
    for (let i = 0; i < R; i++) {
      let r = 0, g = 0, b = 0, a = 0;
      for (let v = 0; v < ny; v++) {
        for (let u = 0; u < nx; u++) {
          // Bilinear sample at the centre of each sub-cell.
          let sx = x + (i * nx + u + 0.5) * sw - 0.5;
          let sy = y + (j * ny + v + 0.5) * sh - 0.5;
          sx = Math.min(W - 1, Math.max(0, sx));
          sy = Math.min(H - 1, Math.max(0, sy));
          const x0 = Math.floor(sx), y0 = Math.floor(sy);
          const x1 = Math.min(W - 1, x0 + 1), y1 = Math.min(H - 1, y0 + 1);
          const fx = sx - x0, fy = sy - y0;
          const w00 = (1 - fx) * (1 - fy), w10 = fx * (1 - fy), w01 = (1 - fx) * fy, w11 = fx * fy;
          const i00 = (y0 * W + x0) * 4, i10 = (y0 * W + x1) * 4, i01 = (y1 * W + x0) * 4, i11 = (y1 * W + x1) * 4;
          if (premult) {
            const a00 = data[i00 + 3] / 255, a10 = data[i10 + 3] / 255, a01 = data[i01 + 3] / 255, a11 = data[i11 + 3] / 255;
            r += data[i00] * a00 * w00 + data[i10] * a10 * w10 + data[i01] * a01 * w01 + data[i11] * a11 * w11;
            g += data[i00 + 1] * a00 * w00 + data[i10 + 1] * a10 * w10 + data[i01 + 1] * a01 * w01 + data[i11 + 1] * a11 * w11;
            b += data[i00 + 2] * a00 * w00 + data[i10 + 2] * a10 * w10 + data[i01 + 2] * a01 * w01 + data[i11 + 2] * a11 * w11;
            a += a00 * w00 + a10 * w10 + a01 * w01 + a11 * w11;
          } else {
            r += data[i00] * w00 + data[i10] * w10 + data[i01] * w01 + data[i11] * w11;
            g += data[i00 + 1] * w00 + data[i10 + 1] * w10 + data[i01 + 1] * w01 + data[i11 + 1] * w11;
            b += data[i00 + 2] * w00 + data[i10 + 2] * w10 + data[i01 + 2] * w01 + data[i11 + 2] * w11;
          }
        }
      }
      const n = nx * ny;
      const o = (j * R + i) * 3;
      rgb[o] = r / n; rgb[o + 1] = g / n; rgb[o + 2] = b / n;
      if (premult) alpha[j * R + i] = a / n;
    }
  }
  return { rgb, alpha };
}

// Decoded 150x150 RGBA piece image -> { P: premultiplied rgb, A: alpha } at R x R.
export function prepareTemplate(img) {
  const { rgb, alpha } = resample(img, 0, 0, img.width, img.height, true);
  return { P: rgb, A: alpha };
}

const medianOf4 = (a, b, c, d) => {
  const s = [a, b, c, d].sort((p, q) => p - q);
  return (s[1] + s[2]) / 2;
};

// Background colour = per-channel median of the four 3x3 corner patches.
function backgroundOf(rgb) {
  const patch = (x0, y0) => {
    let r = 0, g = 0, b = 0;
    for (let y = y0; y < y0 + 3; y++) {
      for (let x = x0; x < x0 + 3; x++) {
        const o = (y * R + x) * 3;
        r += rgb[o]; g += rgb[o + 1]; b += rgb[o + 2];
      }
    }
    return [r / 9, g / 9, b / 9];
  };
  const p = [patch(0, 0), patch(R - 3, 0), patch(0, R - 3), patch(R - 3, R - 3)];
  return [0, 1, 2].map((c) => medianOf4(p[0][c], p[1][c], p[2][c], p[3][c]));
}

// Mean squared error between a cell and "template composited over bg".
// `step` > 1 only looks at every step-th pixel in both directions (fast theme ranking).
function errAgainst(cell, bg, tpl, step = 1) {
  let e = 0, n = 0;
  const { P, A } = tpl;
  for (let y = 0; y < R; y += step) {
    for (let x = 0; x < R; x += step) {
      const k = y * R + x, p = k * 3;
      const inv = 1 - A[k];
      const dr = cell[p] - (P[p] + inv * bg[0]);
      const dg = cell[p + 1] - (P[p + 1] + inv * bg[1]);
      const db = cell[p + 2] - (P[p + 2] + inv * bg[2]);
      e += dr * dr + dg * dg + db * db;
      n++;
    }
  }
  return e / (n * 3);
}

function errEmpty(cell, bg, step = 1) {
  let e = 0, n = 0;
  for (let y = 0; y < R; y += step) {
    for (let x = 0; x < R; x += step) {
      const p = (y * R + x) * 3;
      const dr = cell[p] - bg[0], dg = cell[p + 1] - bg[1], db = cell[p + 2] - bg[2];
      e += dr * dr + dg * dg + db * db;
      n++;
    }
  }
  return e / (n * 3);
}

// Best candidate for one crop within one theme. Returns { piece|null, err, second }.
function matchCell(cell, bg, themeTpl, step = 1) {
  let best = { piece: null, err: errEmpty(cell, bg, step) };
  let second = Infinity;
  for (const code of PIECE_CODES) {
    const e = errAgainst(cell, bg, themeTpl[code], step);
    if (e < best.err) { second = best.err; best = { piece: code, err: e }; }
    else if (e < second) second = e;
  }
  return { ...best, second };
}

function cropCells(img, rect, dx, dy) {
  const cs = rect.size / 8;
  const cells = [];
  for (let r = 0; r < 8; r++) {
    for (let c = 0; c < 8; c++) {
      const { rgb } = resample(img, rect.x + (c + dx) * cs, rect.y + (r + dy) * cs, cs, cs, false);
      cells.push({ rgb, bg: backgroundOf(rgb) });
    }
  }
  return cells;
}

/**
 * @param img        { data, width, height } RGBA screenshot
 * @param rect       { x, y, size } board box in image pixels
 * @param templates  { [theme]: { [pieceCode]: prepared template } }
 * @param opts       { themes?: string[] } restrict the search (e.g. a theme found earlier)
 * @returns { theme, cells: [{ piece, err, second }] (row-major as seen on screen), rmse, themeScores }
 */
export function recognizeBoard(img, rect, templates, opts = {}) {
  const themes = opts.themes && opts.themes.length ? opts.themes.filter((t) => templates[t]) : Object.keys(templates);

  // Stage 1: rank themes using unshifted crops.
  const base = cropCells(img, rect, 0, 0);
  const scores = themes.map((t) => {
    let total = 0;
    for (const c of base) total += matchCell(c.rgb, c.bg, templates[t], 2).err;
    return { theme: t, total };
  });
  scores.sort((a, b) => a.total - b.total);
  const finalists = scores.slice(0, 3).map((s) => s.theme);

  // Stage 2: shifted crops for the finalists.
  const SH = 0.03;
  const shifts = [];
  for (const dy of [-SH, 0, SH]) for (const dx of [-SH, 0, SH]) shifts.push([dx, dy]);
  const crops = shifts.map(([dx, dy]) => (dx === 0 && dy === 0 ? base : cropCells(img, rect, dx, dy)));

  let winner = null;
  for (const theme of finalists) {
    const cells = [];
    let total = 0;
    for (let i = 0; i < 64; i++) {
      let best = null;
      for (const crop of crops) {
        const m = matchCell(crop[i].rgb, crop[i].bg, templates[theme]);
        if (!best || m.err < best.err) best = m;
      }
      cells.push(best);
      total += best.err;
    }
    if (!winner || total < winner.total) winner = { theme, cells, total };
  }

  return {
    theme: winner.theme,
    cells: winner.cells.map(({ piece, err, second }) => ({ piece, err, second })),
    rmse: Math.sqrt(winner.total / 64),
    themeScores: scores.slice(0, 5),
  };
}
