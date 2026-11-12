import { Chess, normalizeMove } from "chessops/chess";
import { makeFen, parseFen, INITIAL_FEN } from "chessops/fen";
import { parseUci } from "chessops/util";
import raw from "@/data/openings.json";
import type { GameRecord } from "@/lib/chess/game";

/**
 * Opening names from the Lichess chess-openings dataset (CC0). Positions are matched by their EPD (board, side to move,
 * castling, en passant), so transpositions get the same name however the moves were ordered.
 */
export type Opening = { eco: string; name: string };
export type BookMove = {
  uci: string;
  /** How many named lines go through this move: a rough measure of how much theory follows. */
  lines: number;
  /** The opening reached by this exact move, if it has a name of its own. */
  reaches?: Opening;
  /** Otherwise, the shortest named line that passes through it. */
  leadsTo?: Opening;
};

type Edge = { lines: number; shortest: number; shortestLen: number };
type Book = {
  names: Map<string, Opening>;
  edges: Map<string, Map<string, Edge>>;
  list: Opening[];
};

const START_FEN_PARTS = INITIAL_FEN.split(" ").slice(0, 4).join(" ");
const keyOf = (pos: Chess) => makeFen(pos.toSetup(), { epd: true });
const freshPos = () => Chess.fromSetup(parseFen(INITIAL_FEN).unwrap()).unwrap();

let cached: Book | null = null;

export function getBook(): Book {
  if (cached) return cached;
  const names = new Map<string, Opening>();
  const edges = new Map<string, Map<string, Edge>>();
  const list: Opening[] = [];
  (raw as [string, string, string][]).forEach(([eco, name, moves], index) => {
    const opening: Opening = { eco, name };
    list.push(opening);
    const pos = freshPos();
    const ucis = moves.split(" ");
    ucis.forEach((uci) => {
      const from = keyOf(pos);
      const m = parseUci(uci)!;
      pos.play(normalizeMove(pos, m));
      let byMove = edges.get(from);
      if (!byMove) edges.set(from, (byMove = new Map()));
      const e = byMove.get(uci);
      if (!e) byMove.set(uci, { lines: 1, shortest: index, shortestLen: ucis.length });
      else {
        e.lines++;
        if (ucis.length < e.shortestLen) { e.shortest = index; e.shortestLen = ucis.length; }
      }
    });
    const end = keyOf(pos);
    if (!names.has(end)) names.set(end, opening);
  });
  cached = { names, edges, list };
  return cached;
}

const isStandardStart = (r: GameRecord) => r.variant === "standard" && r.initialFen.split(" ").slice(0, 4).join(" ") === START_FEN_PARTS;

/** The position key after each of the first `upTo` moves (index 0 = the start). Stops early at an illegal move. */
export function keysAlong(record: GameRecord, upTo: number = record.moves.length): string[] {
  const pos = freshPos();
  const keys = [keyOf(pos)];
  for (let i = 0; i < Math.min(upTo, record.moves.length); i++) {
    const m = parseUci(record.moves[i]);
    if (!m) break;
    const norm = normalizeMove(pos, m);
    if (!pos.isLegal(norm)) break;
    pos.play(norm);
    keys.push(keyOf(pos));
  }
  return keys;
}

/** The most specific named opening reached by the first `ply` moves (default: all), with the ply where it was last matched. */
export function openingAt(record: GameRecord, ply: number = record.moves.length): { opening: Opening; ply: number } | null {
  if (!isStandardStart(record)) return null;
  const { names } = getBook();
  const keys = keysAlong(record, ply);
  for (let i = keys.length - 1; i >= 0; i--) {
    const o = names.get(keys[i]);
    if (o) return { opening: o, ply: i };
  }
  return null;
}

/** Moves that continue a known opening line from the position after `ply` moves, busiest first. */
export function bookMoves(record: GameRecord, ply: number = record.moves.length): BookMove[] {
  if (!isStandardStart(record)) return [];
  const { names, edges, list } = getBook();
  const keys = keysAlong(record, ply);
  if (keys.length - 1 < ply) return [];
  const byMove = edges.get(keys[keys.length - 1]);
  if (!byMove) return [];
  const pos = freshPos();
  for (let i = 0; i < ply; i++) pos.play(normalizeMove(pos, parseUci(record.moves[i])!));
  const out: BookMove[] = [];
  for (const [uci, e] of byMove) {
    const next = pos.clone();
    next.play(normalizeMove(next, parseUci(uci)!));
    const reaches = names.get(keyOf(next));
    out.push({ uci, lines: e.lines, reaches, leadsTo: reaches ? undefined : list[e.shortest] });
  }
  return out.sort((a, b) => b.lines - a.lines || a.uci.localeCompare(b.uci));
}

export const describeOpening = (o: Opening) => `${o.name} (${o.eco})`;
export const openingCount = () => getBook().list.length;
