"use client";

import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { ScreenshotKey } from "@/components/chess/ScreenshotKey";
import { MoveList } from "@/components/chess/MoveList";
import { useBoardSettings } from "@/components/chess/settings";
import {
  describeMoves,
  needsPromotion,
  newRecord,
  toPgn,
  tryMove,
  viewAt,
  type GameRecord,
  type PromotionRole,
  type Result,
} from "@/lib/chess/game";
import { BOTS, BOT_TIERS, candidatesFor, chooseMove, getBot, type Bot } from "@/lib/engine/bots";
import { ClockBar } from "@/components/chess/ClockBar";
import { afterMove, clocksRunning, remaining, TIME_CONTROLS, type Clocks, type Side, type TimeControl } from "./clock";
import { useMoveSounds } from "@/components/useMoveSounds";
import { createClient } from "@/lib/supabase/client";
import { clearSaved, loadSaved, restore, snapshot, writeSaved } from "./saved";
import { useBotBroadcast } from "./useBotBroadcast";
import { useBots } from "./useBots";
import { useEngine } from "./useEngine";

const EMPTY_SANS: string[] = [];

type Setup = { botId: string; color: Side | "random"; variant: "standard" | "chess960"; timeId: string; takebacks: boolean; watchable: boolean };
export type End = { result: Result; how: "checkmate" | "stalemate" | "insufficient" | "fifty" | "threefold" | "variant" | "resignation" | "timeout" | "agreement" };
export type Game = {
  /** Identifies this game for spectating. */
  id: string;
  watchable: boolean;
  bot: Bot;
  player: Side;
  record: GameRecord;
  tc: TimeControl;
  takebacks: boolean;
  clocks: Clocks;
  /** Timestamp when the side to move's clock last started. */
  startedAt: number;
  end: End | null;
  /** The bot's own evaluation after its last search (centipawns, bot's point of view). */
  botEval: number | null;
  startDate: string;
};

const DEFAULT_SETUP: Setup = { botId: "cam", color: "random", variant: "standard", timeId: "unlimited", takebacks: true, watchable: true };

const HOW: Record<End["how"], string> = {
  checkmate: "by checkmate",
  stalemate: "by stalemate",
  insufficient: "by insufficient material",
  fifty: "by the fifty-move rule",
  threefold: "by repetition",
  variant: "by variant rules",
  resignation: "by resignation",
  timeout: "on time",
  agreement: "by agreement",
};

const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40 disabled:hover:bg-surface";
const field = "rounded-md border border-line bg-bg px-3 py-2 text-sm text-ink focus:border-brand focus:outline-none";

export function ComputerGame({ initialBotId }: { initialBotId?: string } = {}) {
  const engineState = useEngine();
  const bots = useBots();
  const [setup, setSetup] = useState<Setup>(() => ({ ...DEFAULT_SETUP, botId: initialBotId ?? DEFAULT_SETUP.botId }));
  const [game, setGame] = useState<Game | null>(null);
  const [now, setNow] = useState(0);
  const [notice, setNotice] = useState<string | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [settings, updateSettings] = useBoardSettings();
  const router = useRouter();
  const engine = engineState.status === "ready" ? engineState.engine : null;

  const view = useMemo(() => (game ? viewAt(game.record) : null), [game]);
  useMoveSounds(view?.sans ?? EMPTY_SANS, !!game?.end || !!view?.outcome);
  const plyCount = game?.record.moves.length ?? 0;
  const end = useMemo<End | null>(
    () => game?.end ?? (view?.outcome ? { result: view.outcome.result, how: view.outcome.reason } : null),
    [game, view],
  );
  const botToMove = !!game && !!view && !end && view.turn !== game.player;
  // ---- keep the game safe: save after every change, restore on the way back in -----------------
  const latest = useRef<Game | null>(null);
  useEffect(() => {
    latest.current = game;
    if (game) writeSaved(snapshot(game, Date.now()));
  }, [game]);
  useEffect(() => {
    const flush = () => latest.current && writeSaved(snapshot(latest.current, Date.now()));
    const onHide = () => document.visibilityState === "hidden" && flush();
    document.addEventListener("visibilitychange", onHide);
    window.addEventListener("pagehide", flush);
    return () => {
      document.removeEventListener("visibilitychange", onHide);
      window.removeEventListener("pagehide", flush);
    };
  }, []);
  useEffect(() => {
    queueMicrotask(() => {
      const saved = loadSaved(Date.now());
      if (!saved || saved.end) return;
      const g = restore(saved, Date.now());
      if (!g) return;
      setGame(g);
      setNotice(`Welcome back. Your game against ${g.bot.name} was restored${g.tc.initialMs > 0 ? " (the clocks were paused while you were away)" : ""}.`);
    });
  }, []);

  // Same account on another device (or cleared storage): the published copy of an unfinished game can be continued.
  const [remote, setRemote] = useState<{ id: string; bot_id: string; bot_name: string; player_color: Side; variant: "standard" | "chess960"; initial_fen: string; moves: string[] } | null>(null);
  useEffect(() => {
    if (game) return;
    let dead = false;
    (async () => {
      const supabase = createClient();
      const { data: sess } = await supabase.auth.getSession();
      const uid = sess.session?.user.id;
      if (!uid || dead || loadSaved(Date.now())) return;
      const since = new Date(Date.now() - 24 * 3600_000).toISOString();
      const { data } = await supabase.from("bot_games").select("id, bot_id, bot_name, player_color, variant, initial_fen, moves")
        .eq("player_id", uid).eq("status", "playing").gt("updated_at", since).order("updated_at", { ascending: false }).limit(1);
      if (!dead) setRemote((data?.[0] as typeof remote) ?? null);
    })().catch(() => {});
    return () => {
      dead = true;
    };
  }, [game]);
  const continueRemote = () => {
    if (!remote) return;
    const bot = getBot(remote.bot_id);
    const tc = TIME_CONTROLS[0];
    if (!bot) return;
    const record: GameRecord = { variant: remote.variant, initialFen: remote.initial_fen, moves: remote.moves };
    try {
      viewAt(record);
    } catch {
      return;
    }
    setNotice(`Continuing your game against ${bot.name} (no clock).`);
    setGame({ id: remote.id, watchable: true, bot, player: remote.player_color, record, tc, takebacks: true, clocks: { white: 0, black: 0 }, startedAt: Date.now(), end: null, botEval: null, startDate: new Date().toISOString().slice(0, 10).replaceAll("-", ".") });
    setRemote(null);
  };

  useBotBroadcast(
    game
      ? { id: game.id, botId: game.bot.id, botName: game.bot.name, botRating: game.bot.rating, player: game.player, variant: game.record.variant === "chess960" ? "chess960" : "standard", initialFen: game.record.initialFen, moves: game.record.moves, end: end ? { result: end.result, how: end.how } : null }
      : null,
    !!game?.watchable,
  );

  // ---- start a game ----------------------------------------------------------------------------
  const start = (s: Setup) => {
    const bot = bots.find((b) => b.id === s.botId) ?? getBot(s.botId) ?? BOTS[0];
    const tc = TIME_CONTROLS.find((t) => t.id === s.timeId) ?? TIME_CONTROLS[0];
    const player: Side = s.color === "random" ? (Math.random() < 0.5 ? "white" : "black") : s.color;
    const base = tc.initialMs;
    engine?.newGame();
    setNotice(null);
    setGame({
      id: crypto.randomUUID(),
      watchable: s.watchable,
      bot,
      player,
      record: newRecord(s.variant),
      tc,
      takebacks: s.takebacks,
      clocks: { white: base, black: base },
      startedAt: Date.now(),
      end: null,
      botEval: null,
      startDate: new Date().toISOString().slice(0, 10).replaceAll("-", "."),
    });
  };

  // ---- applying a move (from the human or from the bot) -----------------------------------------
  const applyRecord = useCallback((record: GameRecord, expectedPly: number, mover: Side, extra?: Partial<Game>) => {
    const t = Date.now();
    setGame((g) => {
      if (!g || g.end || g.record.moves.length !== expectedPly) return g;
      const clocks = g.tc.initialMs > 0 ? afterMove(g.clocks, mover, g.startedAt, t, expectedPly, g.tc.incMs) : g.clocks;
      return { ...g, record, clocks, startedAt: t, ...extra };
    });
  }, []);

  const onMove = useCallback(
    (from: string, to: string, promotion?: PromotionRole) => {
      if (!game || end || !view || view.turn !== game.player) return false;
      const res = tryMove(game.record, from as never, to as never, promotion);
      if (!res.ok) return false;
      setNotice(null);
      applyRecord(res.record, game.record.moves.length, game.player);
      return true;
    },
    [game, end, view, applyRecord],
  );
  const isPromotion = useCallback((from: string, to: string) => (game ? needsPromotion(game.record, from as never, to as never) : false), [game]);

  // ---- the bot's turn ---------------------------------------------------------------------------
  const botKey = game ? `${game.record.moves.length}:${game.record.initialFen}` : "";
  useEffect(() => {
    if (!game || !engine || !botToMove) return;
    const ac = new AbortController();
    let cancelled = false;
    const ply = game.record.moves.length;
    const bot = game.bot;
    const botSide: Side = game.player === "white" ? "black" : "white";
    const timed = game.tc.initialMs > 0;

    (async () => {
      const began = performance.now();
      const botClock = timed ? remaining(game.clocks, botSide, botSide, game.startedAt, Date.now(), ply) : Infinity;
      const cap = timed ? Math.max(60, botClock / 25) : Infinity;
      const movetime = bot.engine.movetime !== undefined || timed ? Math.round(Math.min(bot.engine.movetime ?? 2000, cap)) : undefined;
      let result;
      try {
        result = await engine.search(
          {
            fen: game.record.initialFen,
            moves: game.record.moves,
            depth: bot.engine.depth,
            movetime,
            multipv: candidatesFor(bot, ply),
            elo: bot.engine.elo,
            skill: bot.engine.skill,
          },
          ac.signal,
        );
      } catch {
        return;
      }
      if (cancelled) return;

      const feats = describeMoves(game.record, result.lines.map((l) => l.uci));
      const uci = chooseMove(result.lines, bot, feats, Math.random, { ply }) ?? result.best;
      if (!uci) return;

      // Take a human-looking moment, never less than ~0.45s.
      const wait = Math.max(0, 450 - (performance.now() - began));
      if (wait > 0) await new Promise((r) => setTimeout(r, wait));
      if (cancelled) return;

      const from = uci.slice(0, 2);
      const to = uci.slice(2, 4);
      const promo = uci[4] as "q" | "r" | "b" | "n" | undefined;
      const role = promo ? ({ q: "queen", r: "rook", b: "bishop", n: "knight" } as const)[promo] : undefined;
      const res = tryMove(game.record, from as never, to as never, role);
      if (!res.ok) return;
      const chosen = result.lines.find((l) => l.uci === uci) ?? result.lines[0];
      applyRecord(res.record, ply, botSide, { botEval: chosen ? chosen.cp : null });
    })();

    return () => {
      cancelled = true;
      ac.abort();
    };
    // botKey changes exactly when the position changes; `game` also carries clocks, which must not retrigger this.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [botKey, botToMove, engine, game?.player]);

  // ---- clock ticking and flag fall --------------------------------------------------------------
  const timed = !!game && game.tc.initialMs > 0;
  useEffect(() => {
    if (!timed || end) return;
    const id = setInterval(() => {
      const t = Date.now();
      setNow(t);
      setGame((g) => {
        if (!g || g.end || g.tc.initialMs === 0) return g;
        const v = viewAt(g.record);
        if (v.outcome) return g;
        const left = remaining(g.clocks, v.turn, v.turn, g.startedAt, t, g.record.moves.length);
        if (left > 0 || !clocksRunning(g.record.moves.length)) return g;
        return { ...g, end: { result: v.turn === "white" ? "0-1" : "1-0", how: "timeout" } };
      });
    }, 100);
    return () => clearInterval(id);
  }, [timed, end]);

  // Stop thinking the moment the game ends.
  useEffect(() => {
    if (end) engine?.stop();
  }, [end, engine]);

  // ---- player actions --------------------------------------------------------------------------
  const resign = () => game && !end && setGame({ ...game, end: { result: game.player === "white" ? "0-1" : "1-0", how: "resignation" } });

  const offerDraw = () => {
    if (!game || end) return;
    const calm = game.botEval !== null && game.botEval <= 20 && plyCount >= 20;
    if (calm) setGame({ ...game, end: { result: "1/2-1/2", how: "agreement" } });
    else setNotice(`${game.bot.name} declines the draw offer.`);
  };

  const takeback = () => {
    if (!game || end || !game.takebacks) return;
    const botMovedLast = !botToMove && plyCount > 0;
    // If the bot is still thinking, undo only your move; otherwise undo its reply and your move.
    const remove = botMovedLast ? 2 : 1;
    if (plyCount < remove) return;
    const first = game.player === "white" ? 0 : 1;
    if (plyCount - remove < first) return;
    setGame({ ...game, record: { ...game.record, moves: game.record.moves.slice(0, plyCount - remove) }, startedAt: Date.now(), botEval: null });
    setNotice(null);
  };

  const pgn = () => {
    if (!game) return "";
    const names = game.player === "white" ? { White: "You", Black: game.bot.name } : { White: game.bot.name, Black: "You" };
    return toPgn(game.record, {
      Event: `Casual game vs ${game.bot.name}`,
      Date: game.startDate,
      ...names,
      Result: end?.result ?? "*",
      TimeControl: game.tc.initialMs ? `${game.tc.initialMs / 1000}+${game.tc.incMs / 1000}` : "-",
      Termination: end ? HOW[end.how] : undefined,
    });
  };
  const analyse = (review = false) => {
    try {
      window.sessionStorage.setItem("rookery.import", pgn());
      if (review) window.sessionStorage.setItem("rookery.review", "1");
    } catch {
      /* storage blocked: the analysis board will simply open empty */
    }
    router.push("/play/board");
  };
  const download = () => {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([pgn()], { type: "application/x-chess-pgn" }));
    a.download = `rookery-vs-${game?.bot.id ?? "bot"}.pgn`;
    a.click();
    URL.revokeObjectURL(a.href);
  };

  // ============================== SETUP SCREEN ====================================================
  if (!game || !view) {
    const bot = bots.find((b) => b.id === setup.botId) ?? bots[0] ?? BOTS[0];
    return (
      <div className="grid gap-6">
        {remote && (
          <div className="flex flex-wrap items-center gap-3 rounded-xl border border-brand bg-brand/10 px-4 py-3">
            <span className="font-semibold">You have an unfinished game against {remote.bot_name} ({Math.ceil(remote.moves.length / 2)} moves).</span>
            <button onClick={continueRemote} className="rounded-md bg-brand px-3 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Continue</button>
          </div>
        )}
        <div role="radiogroup" aria-label="Choose your opponent" className="grid gap-6">
          {BOT_TIERS.map((tier) => (
            <section key={tier.label} aria-label={tier.label} className="grid gap-3">
              <h2 className="text-sm font-bold uppercase tracking-wide text-dim">{tier.label}</h2>
              <div className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
                {bots.filter((b) => b.rating >= tier.min && b.rating < tier.max).map((b) => (
                  <button
                    key={b.id}
                    role="radio"
                    aria-checked={setup.botId === b.id}
                    onClick={() => setSetup({ ...setup, botId: b.id })}
                    className={`grid gap-1 rounded-xl border p-4 text-left transition ${setup.botId === b.id ? "border-brand bg-brand/10" : b.special ? "border-warn bg-surface hover:bg-surface-2" : "border-line bg-surface hover:bg-surface-2"}`}
                  >
                    {b.avatarImage ? <img src={b.avatarImage} alt="" width={48} height={48} className="size-12 rounded-md [image-rendering:pixelated]" /> : <span className="text-3xl" aria-hidden>{b.avatar}</span>}
                    <span className="font-bold">{b.name}{b.special && <span className="ml-2 rounded bg-warn/30 px-1.5 py-0.5 text-[10px] font-bold uppercase text-ink">Special</span>}</span>
                    <span className="text-xs font-semibold text-brand">≈ {b.rating}</span>
                    <span className="text-xs text-dim">{b.blurb}</span>
                  </button>
                ))}
              </div>
            </section>
          ))}
        </div>

        <section className="grid max-w-2xl gap-4 rounded-xl border border-line bg-surface p-5" aria-label="Game options">
          <p className="text-sm text-dim">
            <strong className="text-ink">{bot.name}:</strong> “{bot.greeting}” Ratings are approximate, tuned by feel and not measured.
          </p>
          <div className="grid gap-4 sm:grid-cols-2">
            <label className="grid gap-1.5 text-sm font-medium">
              You play
              <select className={field} value={setup.color} onChange={(e) => setSetup({ ...setup, color: e.target.value as Setup["color"] })}>
                <option value="random">Random</option>
                <option value="white">White</option>
                <option value="black">Black</option>
              </select>
            </label>
            <label className="grid gap-1.5 text-sm font-medium">
              Game
              <select className={field} value={setup.variant} onChange={(e) => setSetup({ ...setup, variant: e.target.value as Setup["variant"] })}>
                <option value="standard">Standard</option>
                <option value="chess960">Chess960 (random start)</option>
              </select>
            </label>
            <label className="grid gap-1.5 text-sm font-medium">
              Time
              <select className={field} value={setup.timeId} onChange={(e) => setSetup({ ...setup, timeId: e.target.value })}>
                {TIME_CONTROLS.map((t) => (<option key={t.id} value={t.id}>{t.label}</option>))}
              </select>
            </label>
            <label className="flex items-center gap-2 self-end pb-2 text-sm">
              <input type="checkbox" checked={setup.takebacks} onChange={(e) => setSetup({ ...setup, takebacks: e.target.checked })} className="size-4 accent-[var(--brand)]" />
              Allow takebacks
            </label>
            <label className="flex items-center gap-2 text-sm sm:col-span-2">
              <input type="checkbox" checked={setup.watchable} onChange={(e) => setSetup({ ...setup, watchable: e.target.checked })} className="size-4 accent-[var(--brand)]" />
              Let classmates watch this game (needs a login, shows on the Watch page)
            </label>
          </div>
          <p className="text-xs text-dim">King of the Hill and Three-check are for the analysis board and live games: the computer only plays standard chess and Chess960.</p>
          <div className="flex items-center gap-3">
            <button
              onClick={() => start(setup)}
              disabled={engineState.status !== "ready"}
              className="rounded-md bg-brand px-5 py-2.5 font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50"
            >
              {engineState.status === "ready" ? "Play" : engineState.status === "loading" ? "Loading engine…" : "Engine unavailable"}
            </button>
            {engineState.status === "error" && <p role="alert" className="text-sm text-danger">{engineState.message}</p>}
          </div>
        </section>
      </div>
    );
  }

  // ============================== GAME SCREEN =====================================================
  const opponent: Side = game.player === "white" ? "black" : "white";
  const clockOf = (side: Side) => remaining(game.clocks, view.turn, side, game.startedAt, now || game.startedAt, plyCount);
  const youWon = end && ((end.result === "1-0" && game.player === "white") || (end.result === "0-1" && game.player === "black"));
  const draw = end?.result === "1/2-1/2";

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3">
        <ClockBar label={game.bot.name} avatar={game.bot.avatar} avatarImage={game.bot.avatarImage} ms={clockOf(opponent)} active={!end && view.turn === opponent} timed={timed} thinking={botToMove} />
        <Board
          view={view}
          variant={game.record.variant}
          orientation={game.player}
          movable={end ? "none" : game.player}
          settings={settings}
          isPromotion={isPromotion}
          onMove={onMove}
        />
        <ClockBar label="You" ms={clockOf(game.player)} active={!end && view.turn === game.player} timed={timed} />
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={takeback} disabled={!!end || !game.takebacks || plyCount < (game.player === "white" ? 1 : 2)}>Takeback</button>
          <button className={btn} onClick={offerDraw} disabled={!!end || plyCount < 2}>Offer draw</button>
          <button className={btn} onClick={resign} disabled={!!end || plyCount < 1}>Resign</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>

      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${end ? "border-brand bg-brand/15" : "border-line bg-surface"}`}>
          {end ? (
            <div className="grid gap-1">
              <p className="text-lg font-bold">
                {draw ? "Draw" : youWon ? "You win!" : `${game.bot.name} wins`} <span className="font-normal text-dim">{HOW[end.how]}</span>
              </p>
              <p className="text-sm text-dim">
                {`“${game.bot.name}: ${draw ? game.bot.quips.draw : youWon ? game.bot.quips.lose : game.bot.quips.win}”`}{!draw && !youWon ? " Analyse the game to find where it turned." : ""}
              </p>
            </div>
          ) : (
            <p className="font-semibold">{view.turn === game.player ? "Your move" : `${game.bot.name} is thinking`}{view.check ? " (check)" : ""}</p>
          )}
        </div>
        {notice && <p role="status" className="rounded-md border border-line bg-surface px-3 py-2 text-sm text-dim">{notice}</p>}

        <section className="rounded-xl border border-line bg-surface" aria-label="Move list">
          <MoveList sans={view.sans} ply={view.sans.length} />
        </section>
        <ScreenshotKey />

        {end ? (
          <div className="flex flex-wrap gap-2">
            <button className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi" onClick={() => start({ ...setup, botId: game.bot.id, color: game.player === "white" ? "black" : "white" })}>Rematch (swap colours)</button>
            {plyCount >= 2 && <button className={btn} onClick={() => analyse(true)}>Review with engine</button>}
            <button className={btn} onClick={() => analyse(false)}>Analyse</button>
            <button className={btn} onClick={download}>Download PGN</button>
            <button className={btn} onClick={() => { clearSaved(); setGame(null); }}>New opponent</button>
          </div>
        ) : (
          <p className="text-xs text-dim">Playing {game.record.variant === "chess960" ? "Chess960" : "standard chess"} against {game.bot.name} (≈ {game.bot.rating}).</p>
        )}
      </div>
    </div>
  );
}
