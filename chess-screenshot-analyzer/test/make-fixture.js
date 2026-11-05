// Make a synthetic "board seen from Black's side" screenshot for the e2e test.
import fs from 'node:fs';
import { PNG } from 'pngjs';
import { Chess } from 'chess.js';
import { renderScreenshot } from './synth.js';

const g = new Chess();
for (const m of ['e4', 'e5', 'Nf3', 'Nc6', 'Bc4']) g.move(m);
const grid = g.board().map((row) => row.map((p) => (p ? p.color + p.type : null)));
// Black at the bottom: rotate 180 degrees.
const flipped = grid.slice().reverse().map((row) => row.slice().reverse());
const img = renderScreenshot(flipped, { theme: 'classic', board: 'brown', W: 1640, H: 2360, size: 1280, bx: 180, by: 420, highlight: [[0, 6], [2, 5]], seed: 7 });
const png = new PNG({ width: img.width, height: img.height });
png.data = Buffer.from(img.data);
fs.writeFileSync(process.argv[2], PNG.sync.write(png));
console.log('wrote', process.argv[2], 'expected FEN board:', g.fen().split(' ')[0]);
