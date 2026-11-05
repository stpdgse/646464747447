import { Chess } from 'chess.js';
import { detectBoard } from '../src/detect.js';
import { recognizeBoard } from '../src/recognize.js';
import { renderScreenshot, loadTemplates, themeNames, BOARDS } from './synth.js';

function rng(seed) { let s = seed >>> 0; return () => ((s = (s * 1664525 + 1013904223) >>> 0) / 2 ** 32); }
const rand = rng(12345);
const pick = (a) => a[Math.floor(rand() * a.length)];

function randomGrid() {
  const g = new Chess();
  const plies = 20 + Math.floor(rand() * 50);
  for (let i = 0; i < plies && !g.isGameOver(); i++) { const m = g.moves(); g.move(m[Math.floor(rand() * m.length)]); }
  const grid = g.board().map((row) => row.map((p) => (p ? p.color + p.type : null)));
  return grid;
}

const t0 = Date.now();
const templates = loadTemplates();
console.log(`loaded ${Object.keys(templates).length} themes in ${Date.now() - t0}ms`);

const themes = process.env.THEMES ? process.env.THEMES.split(',') : ['neo', 'classic', 'wood', 'glass', 'alpha', 'game_room', 'ocean', 'metal'];
const sizes = [{ W: 390, H: 844, size: 366, bx: 12, by: 140 }, { W: 828, H: 1792, size: 760.5, bx: 33.5, by: 300 },
  { W: 1640, H: 2360, size: 1280, bx: 180, by: 420 }, { W: 1100, H: 800, size: 640, bx: 60, by: 60 }, { W: 700, H: 700, size: 560, bx: 70.4, by: 55.3 }];
const N = +process.env.N || 40;
let sq = 0, sqBad = 0, boards = 0, boardsBad = 0, detBad = 0, themeBad = 0, maxMs = 0;
for (let n = 0; n < N; n++) {
  const grid = randomGrid();
  const sz = pick(sizes), theme = pick(themes), board = pick(Object.keys(BOARDS));
  const a = Math.floor(rand() * 64), b = Math.floor(rand() * 64);
  const img = renderScreenshot(grid, { ...sz, theme, board, highlight: [[a >> 3, a & 7], [b >> 3, b & 7]], seed: n + 1 });
  const ts = Date.now();
  const det = detectBoard(img);
  const rec = recognizeBoard(img, det, templates);
  maxMs = Math.max(maxMs, Date.now() - ts);
  const err = Math.max(Math.abs(det.x - sz.bx), Math.abs(det.y - sz.by), Math.abs(det.size - sz.size));
  let bad = 0;
  for (let i = 0; i < 64; i++) { sq++; if ((rec.cells[i].piece || null) !== (grid[i >> 3][i & 7] || null)) { bad++; sqBad++; } }
  boards++; if (bad) boardsBad++;
  if (err > 0.03 * sz.size / 8 * 8 * 0.1 + 3) detBad++;
  if (rec.theme !== theme) themeBad++;
  if (bad || err > 4 || rec.theme !== theme) console.log(`#${n} ${theme}/${board} ${sz.W}x${sz.H} det err ${err.toFixed(1)}px str ${det.strength.toFixed(2)} -> theme ${rec.theme} bad squares ${bad} rmse ${rec.rmse.toFixed(1)}`);
}
console.log(`boards ${boards}, boards with errors ${boardsBad}, squares wrong ${sqBad}/${sq}, detection off >3px: ${detBad}, wrong theme: ${themeBad}, slowest detect+recognize ${maxMs}ms`);
process.exit(boardsBad || detBad ? 1 : 0);
