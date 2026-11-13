"use client";

import Link from "next/link";
import { useRouter } from "next/navigation";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { abortGame, claimTimeout, drawAction, makeMove, rematch, resign } from "@/app/actions/games";
import { Board } from "@/components/chess/Board";
import { BoardSettingsPanel } from "@/components/chess/BoardSettingsPanel";
import { ClockBar } from "@/components/chess/ClockBar";
import { ScreenshotKey } from "@/components/chess/ScreenshotKey";
import { MoveList } from "@/components/chess/MoveList";
import { useBoardSettings } from "@/components/chess/settings";
import { needsPromotion, toPgn, tryMove, viewAt, type GameRecord, type PromotionRole } from "@/lib/chess/game";
import { VARIANTS } from "@/lib/chess/variants";
import { categoryOf, describeGameTime, formatDuration } from "@/lib/live/timecontrols";
import { clockLeft, deadlineLeft, needsTimeoutCheck, recordOf, sideOf, type GameRow, type Names, type Side } from "@/lib/live/types";
import { createClient } from "@/lib/supabase/client";
import { signed } from "@/lib/ratings";
import { useMoveFeedback } from "@/components/computer/useMoveFeedback";
import { MoveFeedbackBar } from "@/components/chess/MoveFeedbackBar";
import { usePrefs } from "@/lib/prefs";
import { useMoveSounds } from "@/components/useMoveSounds";
import { ChatBox } from "@/components/social/ChatBox";
import { ReportButton } from "@/components/social/ReportButton";
import { useLiveGame } from "./useLiveGame";

const REASONS: Record<string, string> = {
  checkmate: "by checkmate", stalemate: "by stalemate", insufficient: "by insufficient material", variant: "by variant rules",
  fifty: "by the fifty-move rule", threefold: "by repetition", resignation: "by resignation", timeout: "on time",
  agreement: "by agreement", aborted: "(aborted)", abandoned: "(nobody moved)",
};

const btn = "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40 disabled:hover:bg-surface";

export function LiveGame({ initial, names, viewerId, isMod = false }: { initial: GameRow; names: Names; viewerId: string | null; isMod?: boolean }) {
  const { row, merge, offset, connection } = useLiveGame(initial);
  const router = useRouter();
  const [settings, updateSettings] = useBoardSettings();
  const side = sideOf(row, viewerId);
  const [flipped, setFlipped] = useState(false);
  const [pending, setPending] = useState<GameRecord | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmResign, setConfirmResign] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [rematchSent, setRematchSent] = useState(false);
  const [now, setNow] = useState(0);
  const [changes, setChanges] = useState<Record<string, { before: number; after: number }>>({});

  const orientation: Side = (side ?? "white") === "white" ? (flipped ? "black" : "white") : flipped ? "white" : "black";

  // The board shows our own just-played move straight away; the server's row replaces it when it arrives.
  const waitingForServer = pending !== null && pending.moves.length > row.moves.length;
  const record = useMemo(() => (waitingForServer && pending ? pending : recordOf(row)), [waitingForServer, pending, row]);
  const view = useMemo(() => viewAt(record), [record]);
  const playing = row.status === "started";
  useMoveSounds(view.sans, row.status !== "started");
  // Move feedback (an engine rating of your own moves) is only for casual games: in rated games it would be engine help.
  const [prefs, updatePrefs] = usePrefs();
  const feedbackAllowed = !row.rated && !!side;
  const serverRecord = useMemo(() => recordOf(row), [row]);
  const feedback = useMoveFeedback(feedbackAllowed && side ? { id: row.id, record: serverRecord, player: side } : null, prefs.feedback && feedbackAllowed);

  // ---- rating changes, shown once a rated game has finished -------------------------------------
  const finishedRated = row.status === "finished" && row.rated;
  useEffect(() => {
    if (!finishedRated) return;
    let dead = false;
    createClient()
      .from("rating_history")
      .select("user_id, rating_before, rating_after")
      .eq("game_id", row.id)
      .then(({ data }) => {
        if (dead || !data) return;
        setChanges(Object.fromEntries(data.map((d) => [d.user_id, { before: d.rating_before, after: d.rating_after }])));
      });
    return () => {
      dead = true;
    };
  }, [finishedRated, row.id]);

  // ---- clocks and flag-fall -----------------------------------------------------------------------
  const latest = useRef({ row, offset, viewerId });
  useEffect(() => {
    latest.current = { row, offset, viewerId };
  });
  const lastClaim = useRef(0);
  useEffect(() => {
    if (!playing) return;
    const id = setInterval(() => {
      const t = Date.now();
      setNow(t);
      const cur = latest.current;
      if (cur.viewerId && needsTimeoutCheck(cur.row, t + cur.offset) && t - lastClaim.current > 2500) {
        lastClaim.current = t;
        claimTimeout(cur.row.id).then((r) => r.ok && merge(r.game));
      }
    }, 150);
    return () => clearInterval(id);
  }, [playing, merge]);

  const serverNow = (now || Date.parse(row.clock_updated_at)) + offset;
  const whiteLeft = clockLeft(row, "white", serverNow);
  const blackLeft = clockLeft(row, "black", serverNow);

  // ---- actions ------------------------------------------------------------------------------------
  const onMove = useCallback(
    (from: string, to: string, promotion?: PromotionRole) => {
      if (!side || !playing || waitingForServer) return false;
      const base = recordOf(row);
      if (viewAt(base).turn !== side) return false;
      const attempt = tryMove(base, from as never, to as never, promotion);
      if (!attempt.ok) return false;
      setError(null);
      setPending(attempt.record);
      makeMove(row.id, from, to, promotion).then((res) => {
        if (res.ok) merge(res.game);
        else {
          setPending(null);
          setError(res.error);
        }
      });
      return true;
    },
    [side, playing, waitingForServer, row, merge],
  );
  const isPromotion = useCallback((from: string, to: string) => needsPromotion(record, from as never, to as never), [record]);

  const run = async (fn: () => Promise<{ ok: true; game: GameRow } | { ok: false; error: string }>) => {
    setError(null);
    const res = await fn();
    if (res.ok) merge(res.game);
    else setError(res.error);
  };

  const pgn = () =>
    toPgn(recordOf(row), {
      Event: `Rookery ${row.mode === "daily" ? "daily" : categoryOf(row.time_initial_ms, row.time_inc_ms)} game`,
      Site: "Rookery",
      Date: row.created_at.slice(0, 10).replaceAll("-", "."),
      White: names.white,
      Black: names.black,
      Result: row.result ?? "*",
      TimeControl: row.mode === "daily" ? `1/${(row.days_per_move ?? 1) * 86400}` : `${row.time_initial_ms / 1000}+${row.time_inc_ms / 1000}`,
      Termination: row.reason ?? undefined,
    });
  const analyse = (review = false) => {
    try {
      window.sessionStorage.setItem("rookery.import", pgn());
      if (review) window.sessionStorage.setItem("rookery.review", "1");
    } catch {
      /* storage blocked */
    }
    router.push("/play/board");
  };

  // The opening's name (the list of openings is loaded on demand).
  const [openingName, setOpeningName] = useState<string | null>(null);
  useEffect(() => {
    let dead = false;
    import("@/lib/openings").then((lib) => {
      if (dead) return;
      const hit = lib.openingAt(recordOf(row));
      setOpeningName(hit ? lib.describeOpening(hit.opening) : null);
    });
    return () => { dead = true; };
  }, [row]);
  const download = () => {
    const a = document.createElement("a");
    a.href = URL.createObjectURL(new Blob([pgn()], { type: "application/x-chess-pgn" }));
    a.download = `rookery-${row.id.slice(0, 8)}.pgn`;
    a.click();
    URL.revokeObjectURL(a.href);
  };

  // ---- derived UI state ---------------------------------------------------------------------------
  const other: Side | null = side ? (side === "white" ? "black" : "white") : null;
  const topSide: Side = orientation === "white" ? "black" : "white";
  const bottomSide: Side = orientation;
  const ratingOf = (s: Side) => (s === "white" ? row.white_rating : row.black_rating);
  const nameOf = (s: Side) => (s === "white" ? names.white : names.black);
  const labelOf = (s: Side) => `${nameOf(s)}${ratingOf(s) ? ` (${ratingOf(s)})` : ""}`;
  const changeLine = (s: Side) => {
    const id = s === "white" ? row.white_id : row.black_id;
    const c = changes[id];
    return c ? `${nameOf(s)}: ${c.before} → ${c.after} (${signed(c.after - c.before)})` : null;
  };
  const leftOf = (s: Side) => (s === "white" ? whiteLeft : blackLeft);
  const timeLabel = describeGameTime(row);
  const isDaily = row.mode === "daily";
  const left = deadlineLeft(row, serverNow);
  const noteFor = (s: Side) => (isDaily && playing && view.turn === s && left !== null ? (left <= 0 ? "overdue" : `${formatDuration(left)} left`) : undefined);

  const winnerSide = row.result === "1-0" ? "white" : row.result === "0-1" ? "black" : null;
  const headline =
    row.status === "aborted"
      ? "Game aborted"
      : row.status === "finished"
        ? row.result === "1/2-1/2"
          ? "Draw"
          : side
            ? winnerSide === side
              ? "You win!"
              : "You lose"
            : `${winnerSide === "white" ? names.white : names.black} wins`
        : null;

  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3">
        <ClockBar label={labelOf(topSide)} ms={leftOf(topSide)} active={playing && view.turn === topSide} timed={!isDaily} note={noteFor(topSide)} thinking={false} />
        <Board
          view={view}
          variant={row.variant}
          orientation={orientation}
          movable={side && playing ? side : "none"}
          settings={settings}
          isPromotion={isPromotion}
          onMove={onMove}
        />
        <ClockBar label={labelOf(bottomSide)} ms={leftOf(bottomSide)} active={playing && view.turn === bottomSide} timed={!isDaily} note={noteFor(bottomSide)} />
        {feedbackAllowed ? (
          <MoveFeedbackBar feedback={feedback.latest} pending={feedback.pending} enabled={prefs.feedback} onToggle={() => updatePrefs({ feedback: !prefs.feedback })} hint="Make a move and I'll tell you how good it was." />
        ) : side ? (
          <p className="text-xs text-dim">Move feedback is only available in casual games, not rated ones.</p>
        ) : null}
        <div className="flex flex-wrap gap-2">
          <button className={btn} onClick={() => setFlipped((f) => !f)}>Flip</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>

      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 ${playing ? "border-line bg-surface" : "border-brand bg-brand/15"}`}>
          {playing ? (
            <div className="grid gap-1">
              <p className="font-semibold">
                {side ? (view.turn === side ? "Your move" : `Waiting for ${nameOf(view.turn)}`) : `${nameOf(view.turn)} to move`}
                {view.check ? " (check)" : ""}
              </p>
              <p className="text-xs text-dim">
                {timeLabel} · {VARIANTS[row.variant].label} · {row.rated ? "Rated" : "Casual"}
                {!side && " · you are watching"}
              </p>
            </div>
          ) : (
            <div className="grid gap-1">
              <p className="text-lg font-bold">
                {headline} <span className="font-normal text-dim">{row.reason ? REASONS[row.reason] ?? "" : ""}</span>
              </p>
              <p className="text-xs text-dim">
                {row.result ? `${row.result} · ` : ""}{timeLabel} · {VARIANTS[row.variant].label} · {row.rated ? "Rated" : "Casual"}
              </p>
              {row.rated && row.status === "finished" && (
                <ul className="text-sm" aria-label="Rating changes">
                  {(side ? [side] : (["white", "black"] as Side[])).map((s) => changeLine(s) && <li key={s}>{changeLine(s)}</li>)}
                </ul>
              )}
            </div>
          )}
        </div>

        {connection === "polling" && playing && (
          <p role="status" className="rounded-md border border-line bg-surface px-3 py-2 text-xs text-dim">
            Live connection unavailable on this network. The game still works, with updates every second or two.
          </p>
        )}
        {error && <p role="alert" className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger">{error}</p>}

        {playing && side && other && row.draw_offer_by === other && (
          <div className="flex flex-wrap items-center gap-2 rounded-xl border border-warn/50 bg-warn/10 px-4 py-3 text-sm">
            <span className="font-semibold">{nameOf(other)} offers a draw.</span>
            <button className={btn} onClick={() => run(() => drawAction(row.id, "accept"))}>Accept</button>
            <button className={btn} onClick={() => run(() => drawAction(row.id, "decline"))}>Decline</button>
          </div>
        )}

        {openingName && <p className="text-sm text-dim" aria-label="Opening"><span className="font-semibold text-ink">Opening:</span> {openingName}</p>}

        <section className="rounded-xl border border-line bg-surface" aria-label="Move list">
          <MoveList sans={view.sans} ply={view.sans.length} />
        </section>
        <ScreenshotKey />

        {playing && side && (
          <div className="flex flex-wrap gap-2">
            {row.moves.length < 2 ? (
              <button className={btn} onClick={() => run(() => abortGame(row.id))}>Abort game</button>
            ) : confirmResign ? (
              <>
                <button className="rounded-md border border-danger bg-danger/15 px-3 py-1.5 text-sm font-semibold text-danger" onClick={() => { setConfirmResign(false); run(() => resign(row.id)); }}>Really resign</button>
                <button className={btn} onClick={() => setConfirmResign(false)}>Keep playing</button>
              </>
            ) : (
              <button className={btn} onClick={() => setConfirmResign(true)}>Resign</button>
            )}
            {row.draw_offer_by === side ? (
              <button className={btn} onClick={() => run(() => drawAction(row.id, "cancel"))}>Cancel draw offer</button>
            ) : (
              row.draw_offer_by !== other && <button className={btn} disabled={row.moves.length < 2} onClick={() => run(() => drawAction(row.id, "offer"))}>Offer draw</button>
            )}
          </div>
        )}

        {!playing && (
          <div className="flex flex-wrap gap-2">
            {side && row.status === "finished" && (
              <button
                className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50"
                disabled={rematchSent}
                onClick={async () => {
                  const r = await rematch(row.id);
                  if (r.ok) setRematchSent(true);
                  else setError(r.error);
                }}
              >
                {rematchSent ? "Rematch offered" : "Offer rematch"}
              </button>
            )}
            {row.moves.length >= 2 && <button className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi" onClick={() => analyse(true)}>Review with engine</button>}
            <button className={btn} onClick={() => analyse(false)}>Analyse</button>
            <button className={btn} onClick={download}>Download PGN</button>
            <Link href={isDaily ? "/play/daily" : "/play/online"} className={`${btn} inline-block`}>Back to lobby</Link>
          </div>
        )}

        {viewerId && (
          <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
            {side && (
              <ReportButton viewerId={viewerId} kind="game" refId={row.id} targetUser={side === "white" ? row.black_id : row.white_id}
                label="Report opponent" defaultReason="cheating" />
            )}
            {isMod && row.status === "finished" && (
              <Link href={`/admin?tab=fairplay&game=${row.id}`} className="text-xs text-dim underline hover:text-ink">Fair-play check</Link>
            )}
          </div>
        )}

        <ChatBox table="game_messages" scopeColumn="game_id" scopeId={row.id} room={side ? "players" : "spectators"} viewerId={viewerId} isMod={isMod}
          title={side ? "Chat with your opponent" : "Spectator chat"} maxLength={200} height="h-40" />
      </div>
    </div>
  );
}
