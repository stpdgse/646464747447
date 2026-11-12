"use client";

import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  fromPgn,
  needsPromotion,
  newRecord,
  parseFenOrThrow,
  toPgn,
  tryMove,
  tryText,
  viewAt,
  type GameRecord,
  type PromotionRole,
} from "@/lib/chess/game";
import { VARIANTS, VARIANT_IDS, randomChess960Id, type Variant } from "@/lib/chess/variants";
import { Board } from "./Board";
import { BoardSettingsPanel } from "./BoardSettingsPanel";
import { MoveList } from "./MoveList";
import { useBoardSettings } from "./settings";

const REASONS: Record<string, string> = {
  checkmate: "by checkmate",
  stalemate: "by stalemate",
  insufficient: "by insufficient material",
  variant: "by variant rules",
  fifty: "by the fifty-move rule",
  threefold: "by threefold repetition",
};

const btn =
  "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40 disabled:hover:bg-surface";
const field =
  "rounded-md border border-line bg-bg px-3 py-2 text-sm text-ink placeholder:text-dim focus:border-brand focus:outline-none";

export function Workspace() {
  const [settings, updateSettings] = useBoardSettings();
  const [record, setRecord] = useState<GameRecord>(() => newRecord("standard"));
  const [ply, setPly] = useState(0);
  const [orientation, setOrientation] = useState<"white" | "black">("white");
  const [variant, setVariant] = useState<Variant>("standard");
  const [id960, setId960] = useState("");
  const [moveText, setMoveText] = useState("");
  const [notice, setNotice] = useState<{ kind: "error" | "info"; text: string } | null>(null);
  const [fenInput, setFenInput] = useState("");
  const [pgnInput, setPgnInput] = useState("");
  const [showSettings, setShowSettings] = useState(false);

  const view = useMemo(() => viewAt(record, ply), [record, ply]);
  const atEnd = ply === record.moves.length;
  const live = useMemo(() => viewAt(record), [record]);

  const branch = useCallback((): GameRecord => ({ ...record, moves: record.moves.slice(0, ply) }), [record, ply]);

  const commit = useCallback((next: GameRecord) => {
    setRecord(next);
    setPly(next.moves.length);
    setNotice(null);
  }, []);

  const onMove = useCallback(
    (from: string, to: string, promotion?: PromotionRole) => {
      const res = tryMove(branch(), from as never, to as never, promotion);
      if (!res.ok) return false;
      commit(res.record);
      return true;
    },
    [branch, commit],
  );

  const isPromotion = useCallback((from: string, to: string) => needsPromotion(branch(), from as never, to as never), [branch]);

  const start = (v: Variant, id?: number) => {
    commit(newRecord(v, v === "chess960" ? (id ?? randomChess960Id()) : undefined));
    setVariant(v);
  };

  const startFromForm = () => {
    if (variant === "chess960" && id960.trim() !== "") {
      const n = Number(id960);
      if (!Number.isInteger(n) || n < 0 || n > 959) {
        setNotice({ kind: "error", text: "Chess960 position numbers go from 0 to 959." });
        return;
      }
      start(variant, n);
    } else start(variant);
  };

  // Keyboard navigation (ignored while typing in a field).
  const rootRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      const t = e.target as HTMLElement | null;
      if (t && (t.tagName === "INPUT" || t.tagName === "TEXTAREA" || t.tagName === "SELECT" || t.isContentEditable)) return;
      if (e.key === "ArrowLeft") setPly((p) => Math.max(0, p - 1));
      else if (e.key === "ArrowRight") setPly((p) => Math.min(record.moves.length, p + 1));
      else if (e.key === "ArrowUp") setPly(0);
      else if (e.key === "ArrowDown") setPly(record.moves.length);
      else if (e.key.toLowerCase() === "f") setOrientation((o) => (o === "white" ? "black" : "white"));
      else return;
      e.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [record.moves.length]);

  const submitText = (e: React.FormEvent) => {
    e.preventDefault();
    const res = tryText(branch(), moveText);
    if (res.ok) {
      commit(res.record);
      setMoveText("");
    } else if (res.reason === "promotion") {
      setNotice({ kind: "error", text: 'Add the piece, for example "e8=Q".' });
    } else {
      setNotice({ kind: "error", text: `"${moveText.trim()}" is not a legal move here.` });
    }
  };

  const loadFen = () => {
    try {
      commit(parseFenOrThrow(variant, fenInput));
      setFenInput("");
    } catch (err) {
      setNotice({ kind: "error", text: (err as Error).message });
    }
  };

  const loadPgn = () => {
    try {
      const { record: r } = fromPgn(pgnInput);
      commit(r);
      setVariant(r.variant);
      setPly(0);
      setNotice({ kind: "info", text: `Loaded ${r.moves.length} moves. Use the arrow keys to step through.` });
    } catch (err) {
      setNotice({ kind: "error", text: (err as Error).message });
    }
  };

  const copy = async (text: string, what: string) => {
    try {
      await navigator.clipboard.writeText(text);
      setNotice({ kind: "info", text: `${what} copied.` });
    } catch {
      setNotice({ kind: "error", text: "Could not copy. Select the text and copy it by hand." });
    }
  };

  const status = (() => {
    if (view.outcome) {
      const who = view.outcome.result === "1-0" ? "White wins" : view.outcome.result === "0-1" ? "Black wins" : "Draw";
      return `${who} ${REASONS[view.outcome.reason]}`;
    }
    return `${view.turn === "white" ? "White" : "Black"} to move${view.check ? " (check)" : ""}`;
  })();


  return (
    <div ref={rootRef} className="grid gap-6 lg:grid-cols-[minmax(0,640px)_minmax(0,1fr)]">
      <div className="grid min-w-0 grid-cols-1 content-start gap-3">
        <Board
          view={view}
          variant={record.variant}
          orientation={orientation}
          movable="both"
          settings={settings}
          isPromotion={isPromotion}
          onMove={onMove}
        />
        <div className="flex flex-wrap items-center gap-2" role="toolbar" aria-label="Board controls">
          <button className={btn} onClick={() => setPly(0)} disabled={ply === 0} aria-label="First move">«</button>
          <button className={btn} onClick={() => setPly((p) => Math.max(0, p - 1))} disabled={ply === 0} aria-label="Previous move">‹</button>
          <button className={btn} onClick={() => setPly((p) => Math.min(record.moves.length, p + 1))} disabled={atEnd} aria-label="Next move">›</button>
          <button className={btn} onClick={() => setPly(record.moves.length)} disabled={atEnd} aria-label="Last move">»</button>
          <button className={btn} onClick={() => setOrientation((o) => (o === "white" ? "black" : "white"))}>Flip</button>
          <button className={btn} onClick={() => commit({ ...record, moves: record.moves.slice(0, -1) })} disabled={record.moves.length === 0}>Undo</button>
          <button className={btn} onClick={() => setShowSettings((s) => !s)} aria-expanded={showSettings}>Appearance</button>
        </div>
        {showSettings && <BoardSettingsPanel settings={settings} update={updateSettings} />}
      </div>

      <div className="grid min-w-0 grid-cols-1 content-start gap-4">
        <div aria-live="polite" className={`rounded-xl border px-4 py-3 font-semibold ${view.outcome ? "border-brand bg-brand/15" : "border-line bg-surface"}`}>
          {status}
          {view.remainingChecks && (
            <span className="ml-3 text-sm font-normal text-dim">
              checks to give: White {view.remainingChecks.white}, Black {view.remainingChecks.black}
            </span>
          )}
          {!atEnd && <span className="ml-3 text-sm font-normal text-dim">(viewing move {ply} of {record.moves.length})</span>}
        </div>

        {notice && (
          <p role={notice.kind === "error" ? "alert" : "status"} className={`rounded-md px-3 py-2 text-sm ${notice.kind === "error" ? "border border-danger/50 bg-danger/10 text-danger" : "border border-line bg-surface text-dim"}`}>
            {notice.text}
          </p>
        )}

        <section className="rounded-xl border border-line bg-surface" aria-label="Move list">
          <MoveList sans={live.sans} ply={ply} onSelect={setPly} />
        </section>

        <form onSubmit={submitText} className="flex gap-2">
          <label htmlFor="move-text" className="sr-only">Type a move</label>
          <input id="move-text" value={moveText} onChange={(e) => setMoveText(e.target.value)} placeholder="Type a move: Nf3, O-O, e7e8q" autoComplete="off" autoCapitalize="none" spellCheck={false} className={`${field} flex-1`} />
          <button className={btn}>Play</button>
        </form>

        <section className="grid gap-3 rounded-xl border border-line bg-surface p-4" aria-label="New game">
          <h2 className="text-sm font-semibold">New game</h2>
          <div className="flex flex-wrap gap-2">
            <label htmlFor="variant" className="sr-only">Variant</label>
            <select id="variant" value={variant} onChange={(e) => setVariant(e.target.value as Variant)} className={field}>
              {VARIANT_IDS.map((v) => (<option key={v} value={v}>{VARIANTS[v].label}</option>))}
            </select>
            {variant === "chess960" && (
              <>
                <label htmlFor="id960" className="sr-only">Chess960 position number</label>
                <input id="id960" value={id960} onChange={(e) => setId960(e.target.value)} inputMode="numeric" placeholder="0-959 (blank = random)" className={`${field} w-48`} />
              </>
            )}
            <button className={btn} onClick={startFromForm}>Start</button>
          </div>
          <p className="text-xs text-dim">{VARIANTS[variant].blurb}</p>
        </section>

        <section className="grid gap-2 rounded-xl border border-line bg-surface p-4" aria-label="Position (FEN)">
          <h2 className="text-sm font-semibold">Position (FEN)</h2>
          <code className="break-all rounded bg-bg p-2 text-xs">{view.fen}</code>
          <div className="flex gap-2">
            <button className={btn} onClick={() => copy(view.fen, "FEN")}>Copy FEN</button>
          </div>
          <div className="flex gap-2">
            <label htmlFor="fen" className="sr-only">Paste a FEN</label>
            <input id="fen" value={fenInput} onChange={(e) => setFenInput(e.target.value)} placeholder="Paste a FEN to set up a position" className={`${field} flex-1`} />
            <button className={btn} onClick={loadFen} disabled={!fenInput.trim()}>Load</button>
          </div>
        </section>

        <section className="grid gap-2 rounded-xl border border-line bg-surface p-4" aria-label="Game (PGN)">
          <h2 className="text-sm font-semibold">Game (PGN)</h2>
          <div className="flex gap-2">
            <button className={btn} onClick={() => copy(toPgn(record), "PGN")}>Copy PGN</button>
            <button
              className={btn}
              onClick={() => {
                const blob = new Blob([toPgn(record)], { type: "application/x-chess-pgn" });
                const a = document.createElement("a");
                a.href = URL.createObjectURL(blob);
                a.download = "rookery-game.pgn";
                a.click();
                URL.revokeObjectURL(a.href);
              }}
            >
              Download
            </button>
          </div>
          <label htmlFor="pgn" className="sr-only">Paste a PGN</label>
          <textarea id="pgn" value={pgnInput} onChange={(e) => setPgnInput(e.target.value)} rows={4} placeholder="Paste a PGN to load a game" className={field} />
          <button className={`${btn} justify-self-start`} onClick={loadPgn} disabled={!pgnInput.trim()}>Load PGN</button>
        </section>

        <p className="text-xs text-dim">Keys: ← → step through moves, ↑ ↓ jump to start or end, F flips the board. Right-click and drag to draw arrows.</p>
      </div>
    </div>
  );
}
