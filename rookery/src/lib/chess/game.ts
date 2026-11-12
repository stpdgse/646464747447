import { castlingSide, normalizeMove, type Position } from "chessops/chess";
import { chessgroundDests } from "chessops/compat";
import { makeFen, parseFen } from "chessops/fen";
import { defaultGame, extend, makePgn, parsePgn, startingPosition, type PgnNodeData } from "chessops/pgn";
import { makeSanAndPlay, parseSan } from "chessops/san";
import type { Move, NormalMove, Role, SquareName } from "chessops/types";
import { makeSquare, makeUci, parseUci, squareRank } from "chessops/util";
import { setupPosition } from "chessops/variant";
import { VARIANTS, startFen, type Variant } from "./variants";

/**
 * A game is just its starting position plus a list of UCI moves. Everything else
 * (positions, SAN, legal moves, result) is derived by replaying, so records are plain
 * JSON, easy to store in Postgres, and impossible to get out of sync.
 * Castling is always stored king-takes-own-rook ("e1h1"), which is unambiguous in Chess960.
 */
export type GameRecord = { variant: Variant; initialFen: string; moves: string[] };

export type PromotionRole = "queen" | "rook" | "bishop" | "knight";
export type Result = "1-0" | "0-1" | "1/2-1/2";
export type Reason = "checkmate" | "stalemate" | "insufficient" | "variant" | "fifty" | "threefold";
export type Outcome = { result: Result; reason: Reason };

export type View = {
  fen: string;
  turn: "white" | "black";
  ply: number;
  sans: string[];
  dests: Map<SquareName, SquareName[]>;
  lastMove?: [SquareName, SquareName];
  check: boolean;
  checkSquare?: SquareName;
  outcome?: Outcome;
  /** Three-check only: how many more checks each side still has to give. */
  remainingChecks?: { white: number; black: number };
};

export function newRecord(variant: Variant, chess960Id?: number): GameRecord {
  return { variant, initialFen: startFen(variant, chess960Id), moves: [] };
}

function startPosition(record: GameRecord): Position {
  const setup = parseFen(record.initialFen).unwrap();
  return setupPosition(VARIANTS[record.variant].rules, setup).unwrap();
}

const positionKey = (pos: Position) => makeFen(pos.toSetup(), { epd: true });

function describeOutcome(pos: Position, repetitions: number): Outcome | undefined {
  const o = pos.outcome();
  if (o) {
    const result: Result = o.winner === "white" ? "1-0" : o.winner === "black" ? "0-1" : "1/2-1/2";
    const reason: Reason = pos.isCheckmate()
      ? "checkmate"
      : pos.isStalemate()
        ? "stalemate"
        : pos.isInsufficientMaterial()
          ? "insufficient"
          : "variant";
    return { result, reason };
  }
  if (pos.isInsufficientMaterial()) return { result: "1/2-1/2", reason: "insufficient" };
  if (pos.halfmoves >= 100) return { result: "1/2-1/2", reason: "fifty" };
  if (repetitions >= 3) return { result: "1/2-1/2", reason: "threefold" };
  return undefined;
}

/** Where the king visibly ends up, so a castling highlight shows king movement rather than "king takes rook". */
function displayMove(before: Position, move: NormalMove): [SquareName, SquareName] {
  const side = castlingSide(before, move);
  if (side) {
    const rank = squareRank(move.from);
    const file = side === "h" ? 6 : 2;
    return [makeSquare(move.from), makeSquare(rank * 8 + file)];
  }
  return [makeSquare(move.from), makeSquare(move.to)];
}

/** Replay a record up to `ply` half-moves (default: the end). Throws on a corrupt record. */
export function viewAt(record: GameRecord, ply: number = record.moves.length): View {
  const pos = startPosition(record);
  const sans: string[] = [];
  const seen = new Map<string, number>([[positionKey(pos), 1]]);
  let lastMove: [SquareName, SquareName] | undefined;
  let repetitions = 1;

  const end = Math.max(0, Math.min(ply, record.moves.length));
  for (let i = 0; i < end; i++) {
    const move = parseUci(record.moves[i]);
    if (!move || !("from" in move) || !pos.isLegal(normalizeMove(pos, move))) {
      throw new Error(`Illegal move ${record.moves[i]} at ply ${i + 1}`);
    }
    const norm = normalizeMove(pos, move) as NormalMove;
    lastMove = displayMove(pos, norm);
    sans.push(makeSanAndPlay(pos, norm));
    const key = positionKey(pos);
    repetitions = (seen.get(key) ?? 0) + 1;
    seen.set(key, repetitions);
  }

  const outcome = describeOutcome(pos, repetitions);
  const check = pos.isCheck();
  const king = check ? pos.board.kingOf(pos.turn) : undefined;
  return {
    fen: makeFen(pos.toSetup()),
    turn: pos.turn,
    ply: end,
    sans,
    dests: outcome ? new Map() : chessgroundDests(pos, { chess960: record.variant === "chess960" }),
    lastMove,
    check,
    checkSquare: king === undefined ? undefined : makeSquare(king),
    outcome,
    remainingChecks: pos.remainingChecks ?? undefined,
  };
}

const ROLE_BY_CHAR: Record<string, Role> = { q: "queen", r: "rook", b: "bishop", n: "knight" };

/** True if moving `from`->`to` is a pawn reaching the last rank (so the player must choose a piece). */
export function needsPromotion(record: GameRecord, from: SquareName, to: SquareName): boolean {
  const pos = startPosition(record);
  for (const uci of record.moves) {
    const m = parseUci(uci);
    if (m) pos.play(normalizeMove(pos, m));
  }
  const fromSq = parseUci(`${from}${to}`);
  if (!fromSq || !("from" in fromSq)) return false;
  const piece = pos.board.get(fromSq.from);
  if (!piece || piece.role !== "pawn") return false;
  const rank = squareRank(fromSq.to);
  return (rank === 0 || rank === 7) && pos.isLegal({ from: fromSq.from, to: fromSq.to, promotion: "queen" });
}

export type MoveAttempt =
  | { ok: true; record: GameRecord; san: string }
  | { ok: false; reason: "illegal" | "promotion" | "finished" };

/** Try a move given as squares (what the board gives us). Returns a new record; never mutates. */
export function tryMove(
  record: GameRecord,
  from: SquareName,
  to: SquareName,
  promotion?: PromotionRole,
): MoveAttempt {
  const view = viewAt(record);
  if (view.outcome) return { ok: false, reason: "finished" };

  const pos = startPosition(record);
  for (const uci of record.moves) {
    const m = parseUci(uci);
    if (m) pos.play(normalizeMove(pos, m));
  }

  const raw = parseUci(`${from}${to}`);
  if (!raw || !("from" in raw)) return { ok: false, reason: "illegal" };
  let move: NormalMove = raw;

  const piece = pos.board.get(move.from);
  const lastRank = squareRank(move.to) === 0 || squareRank(move.to) === 7;
  if (piece?.role === "pawn" && lastRank) {
    if (!promotion) return pos.isLegal({ ...move, promotion: "queen" }) ? { ok: false, reason: "promotion" } : { ok: false, reason: "illegal" };
    move = { ...move, promotion };
  }

  const norm = normalizeMove(pos, move) as NormalMove;
  if (!pos.isLegal(norm)) return { ok: false, reason: "illegal" };

  const san = makeSanAndPlay(pos.clone(), norm);
  return { ok: true, record: { ...record, moves: [...record.moves, makeUci(norm)] }, san };
}

/** Same, for UCI strings (engines, the server, tests). Accepts both "e1g1" and "e1h1" for castling. */
export function tryUci(record: GameRecord, uci: string): MoveAttempt {
  const m = parseUci(uci);
  if (!m || !("from" in m)) return { ok: false, reason: "illegal" };
  return tryMove(record, makeSquare(m.from), makeSquare(m.to), m.promotion as PromotionRole | undefined);
}

/** Try a move written in SAN ("Nf3", "O-O", "exd5", "e8=Q") or UCI ("g1f3"). */
export function tryText(record: GameRecord, text: string): MoveAttempt | { ok: false; reason: "unreadable" } {
  const t = text.trim();
  if (!t) return { ok: false, reason: "unreadable" };
  if (/^[a-h][1-8][a-h][1-8][qrbn]?$/i.test(t)) return tryUci(record, t.toLowerCase());
  const pos = startPosition(record);
  for (const uci of record.moves) {
    const m = parseUci(uci);
    if (m) pos.play(normalizeMove(pos, m));
  }
  const move = parseSan(pos, t.replace(/0/g, "O"));
  if (!move || !("from" in move)) return { ok: false, reason: "unreadable" };
  return tryUci(record, makeUci(move));
}

/** Capture / check / promotion flags for candidate moves in the position after `record`. Used by bot personalities. */
export function describeMoves(
  record: GameRecord,
  ucis: string[],
): Map<string, { capture: boolean; check: boolean; promotion: boolean }> {
  const pos = startPosition(record);
  for (const uci of record.moves) {
    const m = parseUci(uci);
    if (m) pos.play(normalizeMove(pos, m));
  }
  const out = new Map<string, { capture: boolean; check: boolean; promotion: boolean }>();
  for (const uci of ucis) {
    const m = parseUci(uci);
    if (!m || !("from" in m)) continue;
    const norm = normalizeMove(pos, m) as NormalMove;
    if (!pos.isLegal(norm)) continue;
    const piece = pos.board.get(norm.from);
    const castling = castlingSide(pos, norm) !== undefined;
    const enPassant = piece?.role === "pawn" && norm.to === pos.epSquare;
    const capture = !castling && (pos.board.has(norm.to) || enPassant);
    const next = pos.clone();
    next.play(norm);
    out.set(uci, { capture, check: next.isCheck(), promotion: norm.promotion !== undefined });
  }
  return out;
}

export function promotionFromChar(ch: string): PromotionRole | undefined {
  const role = ROLE_BY_CHAR[ch.toLowerCase()];
  return role as PromotionRole | undefined;
}

// ---------------------------------------------------------------------------------------------
// PGN

export type PgnHeaders = Partial<Record<"Event" | "Site" | "Date" | "Round" | "White" | "Black" | "Result", string>> &
  Record<string, string | undefined>;

export function toPgn(record: GameRecord, headers: PgnHeaders = {}): string {
  const view = viewAt(record);
  const game = defaultGame<PgnNodeData>();
  const h = game.headers;
  h.set("Event", headers.Event ?? "Casual game");
  h.set("Site", headers.Site ?? "Rookery");
  h.set("Date", headers.Date ?? new Date().toISOString().slice(0, 10).replaceAll("-", "."));
  h.set("Round", headers.Round ?? "-");
  h.set("White", headers.White ?? "?");
  h.set("Black", headers.Black ?? "?");
  h.set("Result", headers.Result ?? view.outcome?.result ?? "*");
  for (const [k, v] of Object.entries(headers)) if (v !== undefined && !h.has(k)) h.set(k, v);

  const variantName = VARIANTS[record.variant].pgn;
  if (variantName) h.set("Variant", variantName);
  // Chess960 and custom starts must carry their FEN so the PGN is self-contained.
  const defaultFen = startFen(record.variant === "chess960" ? "standard" : record.variant);
  if (record.initialFen !== defaultFen) {
    h.set("SetUp", "1");
    h.set("FEN", record.initialFen);
  }

  extend(game.moves, view.sans.map((san) => ({ san })));
  return makePgn(game);
}

export type ParsedPgn = { record: GameRecord; headers: Record<string, string> };

function variantFromHeaders(headers: Map<string, string>, rules: string): Variant {
  const name = (headers.get("Variant") ?? "").toLowerCase().replaceAll(/[\s_-]/g, "");
  if (name === "chess960" || name === "fischerandom" || name === "fischerrandom") return "chess960";
  if (rules === "kingofthehill") return "kingOfTheHill";
  if (rules === "3check") return "threeCheck";
  return "standard";
}

/** Parse the first game of a PGN. Throws a readable Error on bad input. */
export function fromPgn(text: string): ParsedPgn {
  const games = parsePgn(text);
  if (games.length === 0) throw new Error("No game found in that PGN.");
  const game = games[0];
  const start = startingPosition(game.headers);
  if (start.isErr) throw new Error("That PGN has an invalid or unsupported starting position.");

  const pos = start.unwrap();
  const variant = variantFromHeaders(game.headers, pos.rules);
  const record: GameRecord = { variant, initialFen: makeFen(pos.toSetup()), moves: [] };

  let ply = 0;
  for (const node of game.moves.mainline()) {
    ply++;
    const move: Move | undefined = parseSan(pos, node.san);
    if (!move || !("from" in move)) throw new Error(`Illegal or unreadable move "${node.san}" at move ${Math.ceil(ply / 2)}.`);
    record.moves.push(makeUci(move));
    pos.play(move);
  }
  return { record, headers: Object.fromEntries(game.headers) };
}

export function parseFenOrThrow(variant: Variant, fen: string): GameRecord {
  const parsed = parseFen(fen.trim());
  if (parsed.isErr) throw new Error("That is not a valid FEN.");
  const pos = setupPosition(VARIANTS[variant].rules, parsed.unwrap());
  if (pos.isErr) throw new Error("That position is not legal.");
  return { variant, initialFen: makeFen(pos.unwrap().toSetup()), moves: [] };
}
