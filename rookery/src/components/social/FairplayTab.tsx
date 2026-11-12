"use client";

import Link from "next/link";
import { useCallback, useEffect, useRef, useState } from "react";
import { analyseGame, type Review } from "@/lib/analysis/analysis";
import { UciEngine, createWorkerTransport } from "@/lib/engine/uci";
import { recordOf, type GameRow } from "@/lib/live/types";
import { createClient } from "@/lib/supabase/client";
import { fmtDate, friendlyError, ui } from "@/lib/social";

type Watch = { user_id: string; username: string; games: number; wins: number; net_gain: number; reason: string };
type Flag = { id: number; game_id: string; user_id: string; accuracy: number; best_match: number; moves: number; status: "open" | "cleared" | "confirmed"; note: string | null; created_at: string; user: { username: string } | null };
type Side = { id: string; name: string; accuracy: number | null; match: number; moves: number };

const UUID = /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i;

/** Fair-play tools: a watchlist from the database, plus an in-browser Stockfish check of one finished game. */
export function FairplayTab({ initialGame }: { initialGame: string }) {
  const supabase = createClient();
  const [watch, setWatch] = useState<Watch[] | null>(null);
  const [flags, setFlags] = useState<Flag[]>([]);
  const [gameId, setGameId] = useState(initialGame);
  const [progress, setProgress] = useState<string | null>(null);
  const [sides, setSides] = useState<Side[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [note, setNote] = useState("");
  const [saved, setSaved] = useState<string | null>(null);
  const abort = useRef<AbortController | null>(null);

  const load = useCallback(async () => {
    const [w, f] = await Promise.all([
      createClient().rpc("fairplay_watchlist"),
      createClient().from("fairplay_flags").select("id, game_id, user_id, accuracy, best_match, moves, status, note, created_at, user:profiles!fairplay_flags_user_id_fkey(username)").order("created_at", { ascending: false }).limit(100),
    ]);
    setWatch((w.data as Watch[] | null) ?? []);
    setFlags((f.data as unknown as Flag[] | null) ?? []);
  }, []);

  useEffect(() => {
    queueMicrotask(load);
  }, [load]);
  useEffect(() => () => abort.current?.abort(), []);

  const check = async () => {
    const id = gameId.trim();
    if (!UUID.test(id)) return setError("Paste a game id (the long code in the game's address).");
    setError(null); setSides(null); setSaved(null);
    const { data } = await supabase.from("games").select("*, white:profiles!games_white_id_fkey(username), black:profiles!games_black_id_fkey(username)").eq("id", id).maybeSingle();
    if (!data) return setError("No such game.");
    const row = data as GameRow & { white: { username: string }; black: { username: string } };
    if (row.moves.length < 10) return setError("That game is too short to say anything.");
    const ctl = new AbortController();
    abort.current?.abort();
    abort.current = ctl;
    let engine: UciEngine | null = null;
    try {
      setProgress("Starting the engine…");
      engine = new UciEngine(createWorkerTransport());
      await engine.whenReady();
      const review: Review = await analyseGame(engine, recordOf(row), {
        depth: 14, movetime: 300, signal: ctl.signal,
        onProgress: (d, t) => setProgress(`Analysing move ${d} of ${t}…`),
      });
      const side = (who: "white" | "black"): Side => {
        const mine = review.moves.filter((m) => m.mover === who && m.judgement !== "book");
        const hits = mine.filter((m) => m.bestUci && m.uci === m.bestUci).length;
        return { id: who === "white" ? row.white_id : row.black_id, name: who === "white" ? row.white.username : row.black.username, accuracy: review.accuracy[who], match: mine.length ? hits / mine.length : 0, moves: mine.length };
      };
      setSides([side("white"), side("black")]);
    } catch (e) {
      if ((e as Error).name !== "AbortError") setError("The engine check failed. Try again.");
    } finally {
      engine?.terminate();
      setProgress(null);
    }
  };

  const record = async (s: Side) => {
    setError(null);
    const { error: err } = await supabase.rpc("mod_record_fairplay", {
      p_game: gameId.trim(), p_user: s.id, p_accuracy: s.accuracy ?? 0, p_match: s.match, p_moves: s.moves, p_note: note.trim(),
    });
    if (err) return setError(friendlyError(err));
    setSaved(`Flag recorded for ${s.name}.`);
    load();
  };

  const setStatus = async (f: Flag, status: Flag["status"]) => {
    const { error: err } = await supabase.rpc("mod_fairplay_status", { p_flag: f.id, p_status: status, p_note: f.note ?? "" });
    if (err) setError(friendlyError(err)); else load();
  };

  return (
    <div className="grid min-w-0 gap-8">
      {error && <p role="alert" className={ui.err}>{error}</p>}
      {saved && <p role="status" className={ui.info}>{saved}</p>}

      <section aria-labelledby="chk" className={`${ui.card} grid gap-3`}>
        <h2 id="chk" className="text-lg font-bold">Check a game with the engine</h2>
        <p className="text-sm text-dim">
          Runs Stockfish in your browser over every move and shows each player&apos;s accuracy and how often they played the engine&apos;s top move.
          High numbers alone prove nothing: strong players and short forced games score high. Use it as one clue, then decide.
        </p>
        <div className="flex flex-wrap gap-2">
          <input className={`${ui.field} max-w-md`} value={gameId} onChange={(e) => setGameId(e.target.value)} placeholder="Game id" aria-label="Game id" spellCheck={false} />
          <button disabled={!!progress} onClick={check} className={ui.primary}>Run check</button>
          {progress && <button onClick={() => abort.current?.abort()} className={ui.btn}>Cancel</button>}
        </div>
        {progress && <p role="status" className="text-sm text-dim">{progress}</p>}
        {sides && (
          <div className="grid gap-3">
            <ul className="grid gap-2 sm:grid-cols-2">
              {sides.map((s) => (
                <li key={s.id} className="rounded-lg border border-line bg-bg p-3">
                  <p className="font-semibold">{s.name}</p>
                  <p className="text-sm text-dim">Accuracy <strong className="text-ink">{s.accuracy == null ? "n/a" : `${s.accuracy.toFixed(1)}%`}</strong> · top-move match <strong className="text-ink">{Math.round(s.match * 100)}%</strong> · {s.moves} moves</p>
                  <button onClick={() => record(s)} className={`${ui.btn} mt-2`}>Record flag</button>
                </li>
              ))}
            </ul>
            <label className="grid gap-1 text-xs font-medium">Note for the flag (optional)
              <input className={ui.field} value={note} maxLength={500} onChange={(e) => setNote(e.target.value)} />
            </label>
          </div>
        )}
      </section>

      <section aria-labelledby="wl" className="grid gap-2">
        <h2 id="wl" className="text-lg font-bold">Watchlist</h2>
        <p className="text-sm text-dim">Players with at least 8 rated games in 14 days who won 85% or more, or gained 300+ rating points.</p>
        {watch === null ? <div className="h-16 animate-pulse rounded-xl bg-surface" /> : watch.length === 0 ? <p className="text-sm text-dim">Nobody stands out.</p> : (
          <ul className="grid gap-2">
            {watch.map((w) => (
              <li key={w.user_id} className="flex flex-wrap items-center gap-3 rounded-xl border border-line bg-surface px-4 py-3 text-sm">
                <Link href={`/u/${w.username}`} className="font-semibold hover:underline">{w.username}</Link>
                <span className="text-dim">{w.reason} ({w.wins}/{w.games} wins, {w.net_gain >= 0 ? "+" : ""}{w.net_gain})</span>
              </li>
            ))}
          </ul>
        )}
      </section>

      <section aria-labelledby="fl" className="grid gap-2">
        <h2 id="fl" className="text-lg font-bold">Recorded flags</h2>
        {flags.length === 0 ? <p className="text-sm text-dim">None yet.</p> : (
          <ul className="grid gap-2">
            {flags.map((f) => (
              <li key={f.id} className="grid gap-1 rounded-xl border border-line bg-surface px-4 py-3 text-sm">
                <span className="flex flex-wrap items-center gap-x-3">
                  <Link href={`/u/${f.user?.username}`} className="font-semibold hover:underline">{f.user?.username}</Link>
                  <Link href={`/game/${f.game_id}`} className="text-brand hover:underline">game</Link>
                  <span className="text-dim">{f.accuracy.toFixed(1)}% accuracy · {Math.round(f.best_match * 100)}% top moves · {f.moves} moves · {fmtDate(f.created_at)}</span>
                  <span className={`ml-auto rounded px-2 text-xs font-semibold ${f.status === "confirmed" ? "bg-danger/15 text-danger" : f.status === "cleared" ? "bg-brand/15 text-brand" : "bg-surface-2"}`}>{f.status}</span>
                </span>
                {f.note && <span className="text-dim">{f.note}</span>}
                <span className="flex gap-2">
                  {f.status !== "cleared" && <button className={ui.btn} onClick={() => setStatus(f, "cleared")}>Clear</button>}
                  {f.status !== "confirmed" && <button className={ui.btn} onClick={() => setStatus(f, "confirmed")}>Confirm cheating</button>}
                  {f.status !== "open" && <button className={ui.btn} onClick={() => setStatus(f, "open")}>Reopen</button>}
                </span>
              </li>
            ))}
          </ul>
        )}
      </section>
    </div>
  );
}
