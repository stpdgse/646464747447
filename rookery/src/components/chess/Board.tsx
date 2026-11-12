"use client";

import { Chessground } from "@lichess-org/chessground";
import type { Api } from "@lichess-org/chessground/api";
import type { Config } from "@lichess-org/chessground/config";
import type { Key } from "@lichess-org/chessground/types";
import { useCallback, useEffect, useRef, useState, type CSSProperties } from "react";
import type { PromotionRole, View } from "@/lib/chess/game";
import { BOARD_THEMES, type BoardSettings } from "@/lib/chess/themes";
import type { Variant } from "@/lib/chess/variants";
import "./board.css";

type Square = Key;
type Color = "white" | "black";

export type BoardProps = {
  view: View;
  variant: Variant;
  orientation: Color;
  /** Which side(s) the person at this screen may move. */
  movable: Color | "both" | "none";
  settings: BoardSettings;
  /** Does moving from->to need a promotion choice? */
  isPromotion: (from: Square, to: Square) => boolean;
  /** Return true if the move was accepted. A rejected move snaps the piece back. */
  onMove: (from: Square, to: Square, promotion?: PromotionRole) => boolean;
  className?: string;
};

const PROMOTION_CHOICES: { role: PromotionRole; letter: string; label: string }[] = [
  { role: "queen", letter: "Q", label: "Queen" },
  { role: "rook", letter: "R", label: "Rook" },
  { role: "bishop", letter: "B", label: "Bishop" },
  { role: "knight", letter: "N", label: "Knight" },
];

export function Board({ view, variant, orientation, movable, settings, isPromotion, onMove, className }: BoardProps) {
  const hostRef = useRef<HTMLDivElement>(null);
  const apiRef = useRef<Api | null>(null);
  const [pending, setPending] = useState<{ from: Square; to: Square } | null>(null);

  // Latest props for chessground's long-lived event callback.
  const latest = useRef({ view, isPromotion, onMove });
  useEffect(() => {
    latest.current = { view, isPromotion, onMove };
  });

  const configFor = useCallback(
    (): Config => ({
      fen: view.fen,
      orientation,
      turnColor: view.turn,
      check: view.check ? view.turn : false,
      lastMove: view.lastMove,
      coordinates: settings.coordinates,
      coordinatesOnSquares: false,
      animation: { enabled: settings.animation, duration: 180 },
      highlight: { lastMove: settings.highlights, check: settings.highlights },
      movable: {
        free: false,
        color: movable === "none" ? undefined : movable,
        dests: view.dests,
        showDests: settings.highlights,
        rookCastle: variant === "chess960",
      },
      draggable: { enabled: true, showGhost: true },
      selectable: { enabled: true },
      premovable: { enabled: false },
      drawable: { enabled: true, visible: true },
    }),
    [view, orientation, movable, settings, variant],
  );

  // Create once.
  useEffect(() => {
    const el = hostRef.current;
    if (!el) return;
    const api = Chessground(el, {
      ...configFor(),
      movable: {
        ...configFor().movable,
        events: {
          after: (from, to) => {
            const cur = latest.current;
            if (cur.isPromotion(from, to)) {
              setPending({ from, to });
              return;
            }
            if (!cur.onMove(from, to)) apiRef.current?.set({ fen: cur.view.fen, lastMove: cur.view.lastMove });
          },
        },
      },
    });
    apiRef.current = api;
    return () => {
      api.destroy();
      apiRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps -- created once; later changes go through api.set below
  }, []);

  // Keep chessground in step with props. Coordinates are drawn once at creation, so toggling needs a redraw.
  const coordsRef = useRef(settings.coordinates);
  useEffect(() => {
    const api = apiRef.current;
    if (!api) return;
    api.set(configFor());
    if (coordsRef.current !== settings.coordinates) {
      coordsRef.current = settings.coordinates;
      api.redrawAll();
    }
  }, [configFor, settings.coordinates]);

  const revert = useCallback(() => {
    const cur = latest.current.view;
    apiRef.current?.set({ fen: cur.fen, lastMove: cur.lastMove });
  }, []);

  const choose = (role: PromotionRole) => {
    if (!pending) return;
    const ok = latest.current.onMove(pending.from, pending.to, role);
    setPending(null);
    if (!ok) revert();
  };
  const cancel = () => {
    setPending(null);
    revert();
  };

  const theme = BOARD_THEMES.find((t) => t.id === settings.boardTheme) ?? BOARD_THEMES[0];
  const style = { "--sq-light": theme.light, "--sq-dark": theme.dark } as CSSProperties;
  const promoColor = view.turn === "white" ? "w" : "b";

  return (
    <div
      className={`rk-board piece-set-${settings.pieceSet} relative aspect-square w-full select-none ${className ?? ""}`}
      style={style}
    >
      <div ref={hostRef} className="h-full w-full" />
      {pending && (
        <div
          role="dialog"
          aria-label="Choose a promotion piece"
          className="absolute inset-0 z-30 grid place-items-center bg-black/50"
          onKeyDown={(e) => e.key === "Escape" && cancel()}
        >
          <div className="grid gap-3 rounded-xl border border-line bg-surface p-4 shadow-xl">
            <p className="text-center text-sm font-semibold">Promote to</p>
            <div className="flex gap-2">
              {PROMOTION_CHOICES.map((c, i) => (
                <button
                  key={c.role}
                  type="button"
                  autoFocus={i === 0}
                  onClick={() => choose(c.role)}
                  aria-label={c.label}
                  className="grid size-16 place-items-center rounded-lg bg-surface-2 hover:bg-brand/30"
                >
                  {/* eslint-disable-next-line @next/next/no-img-element -- tiny local SVGs */}
                  <img src={`/pieces/${settings.pieceSet}/${promoColor}${c.letter}.svg`} alt="" className="size-14" />
                </button>
              ))}
            </div>
            <button type="button" onClick={cancel} className="text-sm text-dim hover:text-ink">
              Cancel
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
