// Position helpers: screenshot cells -> grid -> FEN, castling inference, validation.
import { Chess } from 'chess.js';

const FEN_CHAR = { wp: 'P', wn: 'N', wb: 'B', wr: 'R', wq: 'Q', wk: 'K', bp: 'p', bn: 'n', bb: 'b', br: 'r', bq: 'q', bk: 'k' };

export const FILES = 'abcdefgh';
export const sq = (r, c) => FILES[c] + (8 - r); // grid row 0 = rank 8

/** cells: 64 entries row-major exactly as drawn on screen.
 *  flipped=true means White is at the top of the screenshot (board seen from Black's side).
 *  Returns grid[row 0 = rank 8][col 0 = file a] of piece codes | null. */
export function gridFromCells(cells, flipped) {
  const grid = [];
  for (let r = 0; r < 8; r++) {
    const row = [];
    for (let c = 0; c < 8; c++) {
      const cell = flipped ? cells[(7 - r) * 8 + (7 - c)] : cells[r * 8 + c];
      row.push(cell.piece || null);
    }
    grid.push(row);
  }
  return grid;
}

/** Same mapping for an index (screen cell index -> grid [r, c]). */
export function cellToGrid(i, flipped) {
  const r = i >> 3, c = i & 7;
  return flipped ? [7 - r, 7 - c] : [r, c];
}

export function inferCastling(grid) {
  const at = (r, c) => grid[r][c];
  return {
    K: at(7, 4) === 'wk' && at(7, 7) === 'wr',
    Q: at(7, 4) === 'wk' && at(7, 0) === 'wr',
    k: at(0, 4) === 'bk' && at(0, 7) === 'br',
    q: at(0, 4) === 'bk' && at(0, 0) === 'br',
  };
}

export function toFen(grid, turn, castling) {
  const rows = grid.map((row) => {
    let s = '', empty = 0;
    for (const p of row) {
      if (!p) { empty++; continue; }
      if (empty) { s += empty; empty = 0; }
      s += FEN_CHAR[p];
    }
    return s + (empty || '');
  });
  const c = (castling.K ? 'K' : '') + (castling.Q ? 'Q' : '') + (castling.k ? 'k' : '') + (castling.q ? 'q' : '');
  return `${rows.join('/')} ${turn} ${c || '-'} - 0 1`;
}

/** Returns { ok, error?, game?, over? }. */
export function checkPosition(fen) {
  let game;
  try {
    game = new Chess(fen);
  } catch (e) {
    return { ok: false, error: String(e.message || e).replace(/^Invalid FEN: ?/, '') || 'Invalid position' };
  }
  // The side that is NOT to move must not be in check (the king could be captured).
  const parts = fen.split(' ');
  parts[1] = parts[1] === 'w' ? 'b' : 'w';
  parts[3] = '-';
  try {
    const other = new Chess(parts.join(' '));
    if (other.inCheck()) {
      return { ok: false, error: `${parts[1] === 'w' ? 'White' : 'Black'} is in check but it is the other side's move. Switch "Whose move".` };
    }
  } catch { /* fine */ }
  if (game.isCheckmate()) return { ok: true, game, over: 'Checkmate' };
  if (game.isStalemate()) return { ok: true, game, over: 'Stalemate' };
  if (game.isInsufficientMaterial()) return { ok: true, game, over: 'Draw (insufficient material)' };
  return { ok: true, game };
}

/** Both kings found on the "wrong" halves => the board is probably flipped the other way. */
export function looksFlipped(grid) {
  let wk = null, bk = null;
  grid.forEach((row, r) => row.forEach((p, c) => { if (p === 'wk') wk = r; if (p === 'bk') bk = r; }));
  if (wk === null || bk === null) return false;
  return wk <= 3 && bk >= 4;
}

/** UCI "e2e4"/"e7e8q" -> { from, to, promotion }. */
export const parseUci = (u) => ({ from: u.slice(0, 2), to: u.slice(2, 4), promotion: u[4] });

/** Convert a UCI principal variation into SAN moves, up to maxPlies. */
export function pvToSan(fen, pv, maxPlies = 10) {
  const g = new Chess(fen);
  const out = [];
  for (const u of pv.slice(0, maxPlies)) {
    try { out.push(g.move(parseUci(u)).san); } catch { break; }
  }
  return out;
}

/** "1. Nf3 Nc6 2. d4" style text, starting at the side to move. */
export function formatPv(fen, san) {
  const [, turn, , , , full] = fen.split(' ');
  let n = parseInt(full, 10) || 1;
  let white = turn === 'w';
  let s = '';
  san.forEach((m, i) => {
    if (white) s += `${n}. ${m} `;
    else s += (i === 0 ? `${n}… ` : '') + `${m} `;
    if (!white) n++;
    white = !white;
  });
  return s.trim();
}
