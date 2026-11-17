/**
 * Bot tournaments: the rules and the schedule, as plain functions so they can be tested without a browser or a database.
 * The browser that runs a tournament asks `nextGame` what to play next; the page draws `knockoutBracket` / `standings`.
 */

export type Participant = { id: string; name: string; rating: number };
export type Format = "knockout" | "roundrobin";
export type Spec = { format: Format; bestOf: 1 | 3; bots: Participant[] };

export type GameRow = {
  id?: string;
  round: number;
  slot: number;
  game_no: number;
  white_bot_id: string;
  black_bot_id: string;
  status: "playing" | "finished";
  result: "1-0" | "0-1" | "1/2-1/2" | null;
  moves?: string[];
  created_at?: string;
};

export type NextGame =
  | { kind: "new"; round: number; slot: number; gameNo: number; whiteId: string; blackId: string }
  | { kind: "resume"; game: GameRow };

/** How many extra games a tied knockout match may add before the higher-rated bot goes through. */
export const EXTRA_GAMES = 2;

export const KNOCKOUT_SIZES = [4, 8, 16] as const;
export const MAX_ROUND_ROBIN = 8;
export const MIN_BOTS = 3;

export const byRating = (a: Participant, b: Participant) => b.rating - a.rating || a.id.localeCompare(b.id);

export function validateSpec(spec: Spec): string | null {
  const n = spec.bots.length;
  if (new Set(spec.bots.map((b) => b.id)).size !== n) return "Each bot can only enter once.";
  if (spec.format === "knockout" && !(KNOCKOUT_SIZES as readonly number[]).includes(n)) return "A knockout needs 4, 8 or 16 bots.";
  if (spec.format === "roundrobin" && (n < MIN_BOTS || n > MAX_ROUND_ROBIN)) return `A round-robin needs ${MIN_BOTS} to ${MAX_ROUND_ROBIN} bots.`;
  return null;
}

/** Standard seeding order for a bracket of `n` (a power of two): 1 plays n, 2 plays n-1, and the top seeds meet as late as possible. */
export function bracketOrder(n: number): number[] {
  let order = [1, 2];
  while (order.length < n) {
    const total = order.length * 2 + 1;
    order = order.flatMap((s) => [s, total - s]);
  }
  return order.slice(0, n);
}

const rounds = (n: number) => Math.log2(n);

/** The bots in round 1 of a knockout, in bracket order (pairs play each other). */
export function firstRound(bots: Participant[]): Participant[] {
  const seeds = [...bots].sort(byRating);
  return bracketOrder(bots.length).map((s) => seeds[s - 1]);
}

export const roundName = (round: number, totalRounds: number): string => {
  const left = totalRounds - round;
  return left === 0 ? "Final" : left === 1 ? "Semi-finals" : left === 2 ? "Quarter-finals" : `Round of ${2 ** (left + 1)}`;
};

// ------------------------------------------------------------------------------------------------ knockout matches

export type MatchState = { a: Participant | null; b: Participant | null; aScore: number; bScore: number; played: number; winner: Participant | null; live: GameRow | null };

const scoreOf = (g: GameRow, id: string): number => {
  if (g.status !== "finished" || !g.result) return 0;
  if (g.result === "1/2-1/2") return 0.5;
  const whiteWon = g.result === "1-0";
  return (g.white_bot_id === id) === whiteWon ? 1 : 0;
};

/** Where one knockout match stands. `bestOf` games are played; a tie adds up to EXTRA_GAMES more; after that the higher rating goes through. */
export function matchState(a: Participant | null, b: Participant | null, games: GameRow[], bestOf: 1 | 3): MatchState {
  const finished = games.filter((g) => g.status === "finished" && g.result);
  const live = games.find((g) => g.status === "playing") ?? null;
  const aScore = a ? finished.reduce((s, g) => s + scoreOf(g, a.id), 0) : 0;
  const bScore = b ? finished.reduce((s, g) => s + scoreOf(g, b.id), 0) : 0;
  const base: MatchState = { a, b, aScore, bScore, played: finished.length, winner: null, live };
  if (!a || !b) return base;
  const need = Math.floor(bestOf / 2) + 1;
  const wins = (id: string) => finished.filter((g) => g.result !== "1/2-1/2" && scoreOf(g, id) === 1).length;
  // Majority of the best-of reached with wins alone (best of 3: two wins).
  if (bestOf === 3 && (wins(a.id) >= need || wins(b.id) >= need)) return { ...base, winner: wins(a.id) >= need ? a : b };
  if (finished.length >= bestOf && aScore !== bScore) return { ...base, winner: aScore > bScore ? a : b };
  if (finished.length >= bestOf + EXTRA_GAMES) return { ...base, winner: byRating(a, b) <= 0 ? a : b };
  return base;
}

/** All matches of a knockout, round by round, with each side filled in once the previous round decides it. */
export function knockoutBracket(spec: Spec, games: GameRow[]): MatchState[][] {
  const first = firstRound(spec.bots);
  const total = rounds(spec.bots.length);
  const out: MatchState[][] = [];
  let entrants: (Participant | null)[] = first;
  for (let r = 1; r <= total; r++) {
    const matches: MatchState[] = [];
    for (let s = 0; s < entrants.length / 2; s++) {
      const a = entrants[2 * s] ?? null, b = entrants[2 * s + 1] ?? null;
      matches.push(matchState(a, b, games.filter((g) => g.round === r && g.slot === s), spec.bestOf));
    }
    out.push(matches);
    entrants = matches.map((m) => m.winner);
  }
  return out;
}

// ------------------------------------------------------------------------------------------------ round robin

export type ScheduledGame = { round: number; slot: number; gameNo: number; whiteId: string; blackId: string };

/** Every pair plays twice, once with each colour. Rounds use the circle method so every bot plays once per round. */
export function roundRobinSchedule(bots: Participant[]): ScheduledGame[] {
  const ids = [...bots].sort(byRating).map((b) => b.id);
  const list: (string | null)[] = ids.length % 2 === 0 ? [...ids] : [...ids, null];
  const n = list.length;
  const out: ScheduledGame[] = [];
  const pairsByRound: [string, string][][] = [];
  for (let r = 0; r < n - 1; r++) {
    const pairs: [string, string][] = [];
    for (let i = 0; i < n / 2; i++) {
      const x = list[i], y = list[n - 1 - i];
      if (x && y) pairs.push([x, y]);
    }
    pairsByRound.push(pairs);
    list.splice(1, 0, list.pop() as string | null);
  }
  for (const leg of [1, 2] as const) {
    pairsByRound.forEach((pairs, r) => pairs.forEach(([x, y], s) => out.push({
      round: r + 1 + (leg - 1) * pairsByRound.length, slot: s, gameNo: 1, whiteId: leg === 1 ? x : y, blackId: leg === 1 ? y : x,
    })));
  }
  return out;
}

export type Standing = { bot: Participant; played: number; wins: number; draws: number; losses: number; points: number };

export function standings(bots: Participant[], games: GameRow[]): Standing[] {
  const table = new Map<string, Standing>(bots.map((b) => [b.id, { bot: b, played: 0, wins: 0, draws: 0, losses: 0, points: 0 }]));
  for (const g of games) {
    if (g.status !== "finished" || !g.result) continue;
    const w = table.get(g.white_bot_id), b = table.get(g.black_bot_id);
    if (!w || !b) continue;
    w.played++; b.played++;
    if (g.result === "1/2-1/2") { w.draws++; b.draws++; w.points += 0.5; b.points += 0.5; }
    else if (g.result === "1-0") { w.wins++; b.losses++; w.points += 1; }
    else { b.wins++; w.losses++; b.points += 1; }
  }
  return [...table.values()].sort((x, y) => y.points - x.points || y.wins - x.wins || byRating(x.bot, y.bot));
}

// ------------------------------------------------------------------------------------------------ what to play next

export function nextGame(spec: Spec, games: GameRow[]): NextGame | null {
  const resume = games.find((g) => g.status === "playing");
  if (resume) return { kind: "resume", game: resume };
  if (spec.format === "roundrobin") {
    const done = new Set(games.map((g) => `${g.round}:${g.slot}:${g.game_no}`));
    const next = roundRobinSchedule(spec.bots).find((s) => !done.has(`${s.round}:${s.slot}:${s.gameNo}`));
    return next ? { kind: "new", ...next } : null;
  }
  const bracket = knockoutBracket(spec, games);
  for (let r = 0; r < bracket.length; r++) {
    for (let s = 0; s < bracket[r].length; s++) {
      const m = bracket[r][s];
      if (m.winner || !m.a || !m.b) continue;
      const gameNo = m.played + 1;
      const aWhite = gameNo % 2 === 1;
      return { kind: "new", round: r + 1, slot: s, gameNo, whiteId: (aWhite ? m.a : m.b).id, blackId: (aWhite ? m.b : m.a).id };
    }
  }
  return null;
}

/** The winner once everything has been played, otherwise null. */
export function champion(spec: Spec, games: GameRow[]): Participant | null {
  if (nextGame(spec, games)) return null;
  if (spec.format === "roundrobin") return games.length ? standings(spec.bots, games)[0].bot : null;
  const final = knockoutBracket(spec, games).at(-1)?.[0];
  return final?.winner ?? null;
}

/** How many games a tournament can take at most (used to stop a tournament from running forever). */
export function maxGames(spec: Spec): number {
  const n = spec.bots.length;
  return spec.format === "roundrobin" ? n * (n - 1) : (n - 1) * (spec.bestOf + EXTRA_GAMES);
}

// ------------------------------------------------------------------------------------------------ Elo for the bot league

export type EloGame = { white_bot_id: string; black_bot_id: string; result: string | null };

/** Bot Elo: everyone starts at their label rating, and every finished tournament game moves it (K = 24). Order matters, so pass games oldest first. */
export function botElo(start: Record<string, number>, games: EloGame[], k = 24): Record<string, { elo: number; games: number }> {
  const out: Record<string, { elo: number; games: number }> = Object.fromEntries(Object.entries(start).map(([id, r]) => [id, { elo: r, games: 0 }]));
  for (const g of games) {
    if (!g.result) continue;
    const w = out[g.white_bot_id], b = out[g.black_bot_id];
    if (!w || !b) continue;
    const expectW = 1 / (1 + 10 ** ((b.elo - w.elo) / 400));
    const sW = g.result === "1-0" ? 1 : g.result === "0-1" ? 0 : 0.5;
    w.elo += k * (sW - expectW);
    b.elo += k * ((1 - sW) - (1 - expectW));
    w.games++; b.games++;
  }
  return out;
}
