import './style.css';
import { detectBoard, snapBoard } from './detect.js';
import { recognizeBoard, prepareTemplate, PIECE_CODES } from './recognize.js';
import { Engine } from './engine.js';
import {
  gridFromCells, cellToGrid, inferCastling, toFen, checkPosition, looksFlipped, pvToSan, formatPv, parseUci, FILES,
} from './board.js';

const THEMES = [
  'neo', 'classic', 'alpha', 'wood', 'game_room', 'glass', 'gothic', 'metal', 'bases', 'neo_wood', 'icy_sea', 'club', 'ocean',
  'newspaper', 'space', 'cases', 'tigers', 'nature', 'dash', 'graffiti', 'light', 'lolz', 'marble', 'vintage', 'bubblegum', 'condal',
];
const MAX_SIDE = 2800; // downscale huge screenshots to keep memory sane on iPads
const DEPTH_CAP_MS = { 12: 4000, 18: 10000, 24: 30000 };
const ARROW_COLORS = ['#9be05a', '#ffb74d', '#64b5f6', '#ba68c8', '#e57373'];
const $ = (id) => document.getElementById(id);
const nextFrame = () => new Promise((r) => requestAnimationFrame(() => setTimeout(r, 0)));
const pieceUrl = (theme, code) => `./pieces/${theme}/${code}.png`;
const prettyTheme = (t) => t.replace(/_/g, ' ');

const state = {
  img: null, // ImageData of the screenshot
  box: null, // { x, y, size } in image pixels
  rec: null, // result of recognizeBoard
  theme: 'neo',
  themeOverride: '',
  playingAs: 'w',
  toMove: 'me',
  depth: 18,
  lines: 3,
  flipManual: null, // null = follow "I play as"
  castlingOff: new Set(),
  edits: new Map(), // grid index (r*8+c) -> piece code | null
  grid: null,
  fen: '',
  valid: false,
  runToken: 0,
  lastLines: [],
  autoFlipNote: '',
};

let engine = null;
let enginePromise = null;
let templatesPromise = null;

/* ------------------------------------------------------------------ engine */

function startEngine() {
  engine = new Engine('./engine/');
  enginePromise = engine.start().then(
    (name) => { $('engineStatus').textContent = `${name} ready · runs offline on this device`; return true; },
    (err) => {
      $('engineStatus').textContent = `Could not start Stockfish: ${err.message}`;
      $('engineStatus').style.color = 'var(--bad)';
      return false;
    },
  );
}

/* --------------------------------------------------------------- templates */

function loadImage(url) {
  return new Promise((resolve, reject) => {
    const im = new Image();
    im.onload = () => resolve(im);
    im.onerror = () => reject(new Error(`Could not load ${url}`));
    im.src = url;
  });
}

function loadTemplates() {
  if (templatesPromise) return templatesPromise;
  templatesPromise = (async () => {
    const c = document.createElement('canvas');
    c.width = c.height = 150;
    const ctx = c.getContext('2d', { willReadFrequently: true });
    const out = {};
    for (const theme of THEMES) {
      const imgs = await Promise.all(PIECE_CODES.map((code) => loadImage(pieceUrl(theme, code))));
      out[theme] = {};
      PIECE_CODES.forEach((code, i) => {
        ctx.clearRect(0, 0, 150, 150);
        ctx.drawImage(imgs[i], 0, 0, 150, 150);
        out[theme][code] = prepareTemplate(ctx.getImageData(0, 0, 150, 150));
      });
    }
    return out;
  })();
  return templatesPromise;
}

/* ------------------------------------------------------------ image intake */

async function decodeFile(file) {
  let bmp;
  try {
    bmp = await createImageBitmap(file);
  } catch {
    const url = URL.createObjectURL(file);
    try { bmp = await loadImage(url); } finally { URL.revokeObjectURL(url); }
  }
  const w0 = bmp.width, h0 = bmp.height;
  const k = Math.min(1, MAX_SIDE / Math.max(w0, h0));
  const canvas = $('shot');
  canvas.width = Math.round(w0 * k);
  canvas.height = Math.round(h0 * k);
  const ctx = canvas.getContext('2d', { willReadFrequently: true });
  ctx.drawImage(bmp, 0, 0, canvas.width, canvas.height);
  bmp.close?.();
  return ctx.getImageData(0, 0, canvas.width, canvas.height);
}

async function loadFile(file) {
  if (!file || !file.type.startsWith('image/')) { alert('Please choose an image file (a screenshot).'); return; }
  $('work').hidden = false;
  $('pick').classList.add('compact');
  setMsg('<span class="spin"></span> Reading screenshot…');
  await nextFrame();
  try {
    state.img = await decodeFile(file);
  } catch (e) {
    setMsg(`Could not read that image: ${e.message}`);
    return;
  }
  state.edits.clear();
  state.castlingOff.clear();
  state.flipManual = null;
  state.autoFlipNote = '';
  $('flipBox').checked = false;
  $('shotCard').scrollIntoView({ behavior: 'smooth', block: 'nearest' });
  await redetect();
}

async function redetect() {
  if (!state.img) return;
  setMsg('<span class="spin"></span> Looking for the board…');
  await nextFrame();
  const det = detectBoard(state.img);
  const { width: W, height: H } = state.img;
  if (det.strength >= 0.6) {
    state.box = { x: det.x, y: det.y, size: det.size };
    $('detectNote').textContent = 'Board found. If the squares don’t line up, drag the green box or its corners.';
  } else {
    const size = Math.min(W, H) * 0.8;
    state.box = { x: (W - size) / 2, y: (H - size) / 2, size };
    $('detectNote').innerHTML = '<b>Couldn’t find the board automatically.</b> Drag the green box and its corners until the grid lines sit on the 8×8 board.';
  }
  layoutBox();
  await recognise({ full: true, firstRun: true });
}

/* --------------------------------------------------------------- crop box */

function layoutBox() {
  const { width: W, height: H } = state.img;
  const b = state.box;
  const el = $('box');
  el.style.left = `${(b.x / W) * 100}%`;
  el.style.top = `${(b.y / H) * 100}%`;
  el.style.width = `${(b.size / W) * 100}%`;
  el.style.height = `${(b.size / H) * 100}%`;
}

function initBoxDrag() {
  const el = $('box');
  const canvas = $('shot');
  let drag = null;
  let timer = null;
  el.addEventListener('pointerdown', (e) => {
    e.preventDefault();
    const rect = canvas.getBoundingClientRect();
    drag = { mode: e.target.dataset.h || 'move', sx: e.clientX, sy: e.clientY, b: { ...state.box }, k: state.img.width / rect.width };
    el.setPointerCapture(e.pointerId);
  });
  el.addEventListener('pointermove', (e) => {
    if (!drag) return;
    const { width: W, height: H } = state.img;
    const dx = (e.clientX - drag.sx) * drag.k, dy = (e.clientY - drag.sy) * drag.k;
    const b = drag.b;
    let { x, y, size } = b;
    switch (drag.mode) {
      case 'move': x = b.x + dx; y = b.y + dy; break;
      case 'br': size = b.size + (dx + dy) / 2; break;
      case 'tl': size = b.size - (dx + dy) / 2; x = b.x + b.size - size; y = b.y + b.size - size; break;
      case 'tr': size = b.size + (dx - dy) / 2; y = b.y + b.size - size; break;
      case 'bl': size = b.size + (dy - dx) / 2; x = b.x + b.size - size; break;
      default: break;
    }
    size = Math.max(64, Math.min(size, Math.min(W, H)));
    if (drag.mode === 'move') { x = Math.max(0, Math.min(W - size, x)); y = Math.max(0, Math.min(H - size, y)); }
    state.box = { x, y, size };
    layoutBox();
    clearTimeout(timer);
    timer = setTimeout(() => recognise({}), 350);
  });
  const end = () => { drag = null; };
  el.addEventListener('pointerup', end);
  el.addEventListener('pointercancel', end);

  // 8x8 grid lines inside the box
  let d = '';
  for (let i = 1; i < 8; i++) d += `M${i} 0V8M0 ${i}H8`;
  $('gridPath').setAttribute('d', d);
}

/* ------------------------------------------------------------ recognition */

function setMsg(html) { $('msg').innerHTML = html; }

async function recognise({ full = false, firstRun = false } = {}) {
  if (!state.img || !state.box) return;
  state.runToken++; // cancel pending display of older analysis
  engine?.stop?.();
  setMsg('<span class="spin"></span> Reading the pieces…');
  await nextFrame();
  const templates = await loadTemplates();
  if (!full && !firstRun) {
    // A hand-placed box is rarely pixel-exact: snap it onto the real grid lines first.
    state.box = snapBoard(state.img, state.box);
    layoutBox();
  }
  let themes;
  if (state.themeOverride) themes = [state.themeOverride];
  else if (!full && state.theme) themes = [state.theme];
  let rec = recognizeBoard(state.img, state.box, templates, { themes });
  if (themes && !state.themeOverride && rec.rmse > 22) rec = recognizeBoard(state.img, state.box, templates);
  state.rec = rec;
  state.theme = rec.theme;
  state.edits.clear();
  $('themeNote').textContent = `· ${prettyTheme(rec.theme)} pieces`;
  $('themeSel').value = state.themeOverride;

  if (firstRun) {
    // If both kings sit on the "wrong" halves, the screenshot is flipped relative to our assumption.
    const g = gridFromCells(rec.cells, effectiveFlipped());
    if (looksFlipped(g)) {
      state.flipManual = !effectiveFlipped();
      state.autoFlipNote = 'Board direction was corrected automatically: the kings were on the wrong sides.';
    }
  }
  rebuild();
}

const effectiveFlipped = () => (state.flipManual !== null ? state.flipManual : state.playingAs === 'b');

const unsure = (cell) => cell.err > 600 || (cell.piece && cell.second < cell.err * 1.6 && cell.err > 150);

/* ----------------------------------------------------------- position/UI */

function currentTurn() {
  return state.toMove === 'me' ? state.playingAs : state.playingAs === 'w' ? 'b' : 'w';
}

function rebuild() {
  if (!state.rec) return;
  const flipped = effectiveFlipped();
  $('flipBox').checked = flipped;
  const grid = gridFromCells(state.rec.cells, flipped);
  for (const [idx, val] of state.edits) grid[idx >> 3][idx & 7] = val;
  state.grid = grid;

  const inferred = inferCastling(grid);
  const castling = {};
  document.querySelectorAll('#castling button').forEach((b) => {
    const k = b.dataset.k;
    castling[k] = inferred[k] && !state.castlingOff.has(k);
    b.classList.toggle('on', castling[k]);
    b.disabled = !inferred[k];
    b.style.opacity = inferred[k] ? '' : '.35';
  });

  state.fen = toFen(grid, currentTurn(), castling);
  const chk = checkPosition(state.fen);
  state.valid = chk.ok && !chk.over;
  $('fen').textContent = state.fen;
  $('lichess').href = `https://lichess.org/analysis/${state.fen.replace(/ /g, '_')}`;
  $('resetEdits').hidden = state.edits.size === 0;

  const notice = $('notice');
  notice.className = 'notice';
  if (!chk.ok) {
    notice.classList.add('bad');
    notice.textContent = `This position isn't legal: ${chk.error}`;
    notice.hidden = false;
  } else if (state.autoFlipNote) {
    notice.textContent = state.autoFlipNote;
    notice.hidden = false;
  } else {
    notice.hidden = true;
  }

  renderBoard(grid);
  $('arrows').innerHTML = '';
  state.lastLines = [];
  $('lines').innerHTML = '';
  $('evalBar').hidden = true;
  $('depthNote').textContent = '';

  if (!chk.ok) {
    setMsg('Fix the position (tap squares to correct them) and analysis will start.');
    $('againBtn').disabled = true;
    return;
  }
  $('againBtn').disabled = false;
  if (chk.over) {
    setMsg(`<b>${chk.over}</b> — there are no moves to suggest in this position.`);
    return;
  }
  setMsg('<span class="spin"></span> Stockfish is thinking…');
  scheduleAnalysis(250);
}

function viewRC(vr, vc) { return state.playingAs === 'w' ? [vr, vc] : [7 - vr, 7 - vc]; }

function renderBoard(grid) {
  const boardEl = $('board');
  boardEl.innerHTML = '';
  const flipped = effectiveFlipped();
  const uncertain = new Set();
  state.rec.cells.forEach((cell, i) => {
    if (!unsure(cell)) return;
    const [r, c] = cellToGrid(i, flipped);
    uncertain.add(r * 8 + c);
  });
  for (let vr = 0; vr < 8; vr++) {
    for (let vc = 0; vc < 8; vc++) {
      const [r, c] = viewRC(vr, vc);
      const idx = r * 8 + c;
      const b = document.createElement('button');
      b.type = 'button';
      b.className = (r + c) % 2 === 0 ? 'l' : '';
      if (uncertain.has(idx) && !state.edits.has(idx)) b.classList.add('unsure');
      if (state.edits.has(idx)) b.classList.add('edited');
      b.dataset.idx = idx;
      b.setAttribute('aria-label', `${FILES[c]}${8 - r}`);
      const p = grid[r][c];
      if (p) {
        const img = document.createElement('img');
        img.src = pieceUrl(state.theme, p);
        img.alt = p;
        b.appendChild(img);
      }
      if (vc === 0) b.insertAdjacentHTML('beforeend', `<span class="co r">${8 - r}</span>`);
      if (vr === 7) b.insertAdjacentHTML('beforeend', `<span class="co f">${FILES[c]}</span>`);
      boardEl.appendChild(b);
    }
  }
}

function initBoardTaps() {
  const dlg = $('palette');
  let target = null;
  $('board').addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    target = +b.dataset.idx;
    $('palTitle').textContent = `What is on ${FILES[target & 7]}${8 - (target >> 3)}?`;
    const g = $('palGrid');
    g.innerHTML = '';
    for (const code of PIECE_CODES) {
      const pb = document.createElement('button');
      pb.value = code;
      pb.innerHTML = `<img src="${pieceUrl(state.theme, code)}" alt="${code}">`;
      g.appendChild(pb);
    }
    dlg.showModal();
  });
  dlg.addEventListener('close', () => {
    const v = dlg.returnValue;
    dlg.returnValue = '';
    if (target === null || !v || v === 'cancel') return;
    state.edits.set(target, v === 'empty' ? null : v);
    target = null;
    rebuild();
  });
}

/* --------------------------------------------------------------- analysis */

let analysisTimer = null;
function scheduleAnalysis(ms = 300) {
  clearTimeout(analysisTimer);
  analysisTimer = setTimeout(runAnalysis, ms);
}

async function runAnalysis() {
  if (!state.valid) return;
  const ok = await enginePromise;
  if (!ok) { setMsg('Stockfish could not start in this browser.'); return; }
  const token = ++state.runToken;
  const fen = state.fen;
  const depth = state.depth;
  $('stopBtn').hidden = false;
  setMsg('<span class="spin"></span> Stockfish is thinking…');
  let res;
  try {
    res = await engine.analyze(
      { fen, depth, multipv: state.lines, movetime: DEPTH_CAP_MS[depth] },
      (lines, d, final) => { if (token === state.runToken) queueRender(fen, lines, d, final); },
    );
  } catch (e) {
    if (token === state.runToken) setMsg(`Engine error: ${e.message}`);
    return;
  }
  if (token !== state.runToken) return;
  $('stopBtn').hidden = true;
  setMsg(res.stopped ? 'Stopped.' : `Done · searched to depth ${res.depth}.`);
}

let renderQueued = false;
let pending = null;
function queueRender(fen, lines, depth, final) {
  pending = { fen, lines, depth, final };
  if (renderQueued) return;
  renderQueued = true;
  requestAnimationFrame(() => {
    renderQueued = false;
    const p = pending;
    if (p && p.fen === state.fen) renderLines(p.fen, p.lines, p.depth, p.final);
  });
}

function fmtEval(whiteCp, whiteMate) {
  if (whiteMate !== null) return `${whiteMate > 0 ? '' : '-'}M${Math.abs(whiteMate)}`;
  const v = whiteCp / 100;
  return `${v > 0 ? '+' : ''}${v.toFixed(2)}`;
}

function renderLines(fen, lines, depth, final) {
  state.lastLines = lines;
  const turn = fen.split(' ')[1];
  const sign = turn === 'w' ? 1 : -1;
  $('depthNote').textContent = lines.length ? `· depth ${depth}${final ? '' : '…'}` : '';
  const html = lines.map((l, i) => {
    const san = pvToSan(fen, l.pv, 10);
    if (!san.length) return '';
    const mv = parseUci(l.pv[0]);
    const wc = l.cp !== null ? l.cp * sign : null;
    const wm = l.mate !== null ? l.mate * sign : null;
    return `<div class="line ${i === 0 ? 'best' : ''}" style="--c:${ARROW_COLORS[i % 5]}">
      <span class="mv">${i === 0 ? '★ ' : ''}${san[0]}</span>
      <span class="sub">${mv.from} → ${mv.to}</span>
      <span class="ev">${fmtEval(wc, wm)}</span>
      <span class="pv">${formatPv(fen, san)}</span></div>`;
  }).join('');
  $('lines').innerHTML = html;

  if (lines[0]) {
    const l = lines[0];
    const wc = l.cp !== null ? l.cp * sign : null;
    const wm = l.mate !== null ? l.mate * sign : null;
    const frac = wm !== null ? (wm > 0 ? 1 : 0) : 1 / (1 + Math.exp(-wc / 400));
    $('evalBar').hidden = false;
    $('evalFill').style.width = `${(frac * 100).toFixed(1)}%`;
    $('evalText').textContent = `White ${fmtEval(wc, wm)}`;
  }
  drawArrows(lines);
}

function drawArrows(lines) {
  const svg = $('arrows');
  let out = '';
  const ctr = (sq) => {
    const c = FILES.indexOf(sq[0]);
    const r = 8 - parseInt(sq[1], 10);
    // find view row/col
    const vr = state.playingAs === 'w' ? r : 7 - r;
    const vc = state.playingAs === 'w' ? c : 7 - c;
    return [vc + 0.5, vr + 0.5];
  };
  document.querySelectorAll('#board button.mv').forEach((b) => b.classList.remove('mv'));
  for (let i = lines.length - 1; i >= 0; i--) {
    const u = lines[i].pv?.[0];
    if (!u) continue;
    const [x1, y1] = ctr(u.slice(0, 2));
    const [x2, y2] = ctr(u.slice(2, 4));
    const dx = x2 - x1, dy = y2 - y1;
    const len = Math.hypot(dx, dy) || 1;
    const ux = dx / len, uy = dy / len;
    const w = i === 0 ? 0.17 : 0.12;
    const head = 0.42;
    const sx = x1 + ux * 0.22, sy = y1 + uy * 0.22;
    const bx = x2 - ux * head, by = y2 - uy * head;
    const px = -uy, py = ux;
    const hw = w * 1.9;
    const col = ARROW_COLORS[i % 5];
    const op = i === 0 ? 0.92 : 0.7;
    out += `<g opacity="${op}"><line x1="${sx}" y1="${sy}" x2="${bx}" y2="${by}" stroke="${col}" stroke-width="${w}" stroke-linecap="round"/>
      <polygon fill="${col}" points="${x2 - ux * 0.05},${y2 - uy * 0.05} ${bx + px * hw},${by + py * hw} ${bx - px * hw},${by - py * hw}"/></g>`;
  }
  svg.innerHTML = out;
  if (lines[0]?.pv?.[0]) {
    const u = lines[0].pv[0];
    for (const sq of [u.slice(0, 2), u.slice(2, 4)]) {
      const c = FILES.indexOf(sq[0]);
      const r = 8 - parseInt(sq[1], 10);
      const b = document.querySelector(`#board button[data-idx="${r * 8 + c}"]`);
      b?.classList.add('mv');
    }
  }
}

/* ---------------------------------------------------------------- controls */

function bindSeg(id, cb) {
  $(id).addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b) return;
    $(id).querySelectorAll('button').forEach((x) => x.classList.toggle('on', x === b));
    cb(b.dataset.v);
  });
}

function initControls() {
  bindSeg('segPlay', (v) => { state.playingAs = v; rebuild(); });
  bindSeg('segMove', (v) => { state.toMove = v; rebuild(); });
  bindSeg('segDepth', (v) => { state.depth = +v; if (state.valid) scheduleAnalysis(50); });
  bindSeg('segLines', (v) => { state.lines = +v; if (state.valid) scheduleAnalysis(50); });

  $('flipBox').addEventListener('change', (e) => { state.autoFlipNote = ''; state.flipManual = e.target.checked; recogniseKeepEdits(); });
  $('castling').addEventListener('click', (e) => {
    const b = e.target.closest('button');
    if (!b || b.disabled) return;
    const k = b.dataset.k;
    if (state.castlingOff.has(k)) state.castlingOff.delete(k); else state.castlingOff.add(k);
    rebuild();
  });
  $('resetEdits').addEventListener('click', () => { state.edits.clear(); rebuild(); });
  $('redetect').addEventListener('click', () => redetect());
  $('stopBtn').addEventListener('click', () => { engine?.stop(); });
  $('againBtn').addEventListener('click', () => { if (state.valid) scheduleAnalysis(0); });
  $('copyFen').addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(state.fen); $('copyFen').textContent = 'Copied ✓'; }
    catch { window.prompt('Copy this FEN:', state.fen); }
    setTimeout(() => { $('copyFen').textContent = 'Copy FEN'; }, 1500);
  });

  const sel = $('themeSel');
  for (const t of THEMES) sel.insertAdjacentHTML('beforeend', `<option value="${t}">${prettyTheme(t)}</option>`);
  sel.addEventListener('change', () => { state.themeOverride = sel.value; recognise({ full: true }); });
}

// Flipping only changes how cells map to squares; keep the existing reading but drop stale edits.
function recogniseKeepEdits() { state.edits.clear(); rebuild(); }

function initIntake() {
  const input = $('file');
  input.addEventListener('change', () => { if (input.files[0]) loadFile(input.files[0]); input.value = ''; });
  const pick = $('pick');
  ['dragenter', 'dragover'].forEach((ev) => document.addEventListener(ev, (e) => { e.preventDefault(); pick.classList.add('drag'); }));
  ['dragleave', 'drop'].forEach((ev) => document.addEventListener(ev, (e) => { e.preventDefault(); pick.classList.remove('drag'); }));
  document.addEventListener('drop', (e) => { const f = e.dataTransfer?.files?.[0]; if (f) loadFile(f); });
  document.addEventListener('paste', (e) => {
    const item = [...(e.clipboardData?.items || [])].find((i) => i.type.startsWith('image/'));
    if (item) loadFile(item.getAsFile());
  });
}

/* -------------------------------------------------------------------- boot */

initControls();
initIntake();
initBoxDrag();
initBoardTaps();
startEngine();
loadTemplates().catch((e) => console.warn('template preload failed', e));

if ('serviceWorker' in navigator && location.protocol.startsWith('http') && import.meta.env.PROD) {
  navigator.serviceWorker.register('./sw.js').catch(() => {});
}

// Test hook (used by the end-to-end test).
window.__app = { state, loadFile };
