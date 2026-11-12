import type { Rules } from "chessops/types";
import { INITIAL_FEN } from "chessops/fen";

export type Variant = "standard" | "chess960" | "kingOfTheHill" | "threeCheck";

export const VARIANTS: Record<Variant, { label: string; rules: Rules; pgn: string | null; blurb: string }> = {
  standard: { label: "Standard", rules: "chess", pgn: null, blurb: "Classic chess." },
  chess960: {
    label: "Chess960",
    rules: "chess",
    pgn: "Chess960",
    blurb: "Shuffled back rank (960 possible starts). Castling works from any start.",
  },
  kingOfTheHill: {
    label: "King of the Hill",
    rules: "kingofthehill",
    pgn: "King of the Hill",
    blurb: "Win by checkmate, or by walking your king to the centre.",
  },
  threeCheck: {
    label: "Three-check",
    rules: "3check",
    pgn: "Three-check",
    blurb: "Win by checkmate, or by giving check three times.",
  },
};

export const VARIANT_IDS = Object.keys(VARIANTS) as Variant[];

export function isVariant(value: unknown): value is Variant {
  return typeof value === "string" && value in VARIANTS;
}

/** Which of the 3 remaining empty squares each knight pair uses, in Scharnagl's numbering. */
const KNIGHT_PAIRS: readonly (readonly [number, number])[] = [
  [0, 1], [0, 2], [0, 3], [0, 4], [1, 2], [1, 3], [1, 4], [2, 3], [2, 4], [3, 4],
];

/**
 * Chess960 starting position by Scharnagl number (0-959). Number 518 is the classical setup.
 * Returned as a Shredder-FEN so castling rights name the exact rook files.
 */
export function chess960Fen(id: number): string {
  if (!Number.isInteger(id) || id < 0 || id > 959) throw new RangeError(`Chess960 id must be 0-959, got ${id}`);
  const rank: string[] = Array(8).fill("");
  let n = id;

  rank[1 + 2 * (n % 4)] = "B"; // light-squared bishop (b, d, f, h)
  n = Math.floor(n / 4);
  rank[2 * (n % 4)] = "B"; // dark-squared bishop (a, c, e, g)
  n = Math.floor(n / 4);

  const empty = () => rank.map((p, i) => (p === "" ? i : -1)).filter((i) => i >= 0);

  rank[empty()[n % 6]] = "Q";
  n = Math.floor(n / 6);

  const free = empty();
  const [a, b] = KNIGHT_PAIRS[n];
  rank[free[a]] = "N";
  rank[free[b]] = "N";

  const last = empty(); // three squares left: rook, king, rook
  rank[last[0]] = "R";
  rank[last[1]] = "K";
  rank[last[2]] = "R";

  const white = rank.join("");
  const files = "abcdefgh";
  const rookFiles = [last[0], last[2]].map((i) => files[i]);
  const castling = `${rookFiles[1].toUpperCase()}${rookFiles[0].toUpperCase()}${rookFiles[1]}${rookFiles[0]}`;
  return `${white.toLowerCase()}/pppppppp/8/8/8/8/PPPPPPPP/${white} w ${castling} - 0 1`;
}

export function randomChess960Id(random: () => number = Math.random): number {
  return Math.floor(random() * 960);
}

/** Start FEN for a variant. `chess960Id` only matters for Chess960 (random when omitted). */
export function startFen(variant: Variant, chess960Id?: number): string {
  if (variant === "chess960") return chess960Fen(chess960Id ?? randomChess960Id());
  if (variant === "threeCheck") return `${INITIAL_FEN.split(" ").slice(0, 4).join(" ")} 3+3 0 1`;
  return INITIAL_FEN;
}
