"use client";

import { BOARD_THEMES, PIECE_SETS, type BoardSettings } from "@/lib/chess/themes";

export function BoardSettingsPanel({
  settings,
  update,
}: {
  settings: BoardSettings;
  update: (patch: Partial<BoardSettings>) => void;
}) {
  const toggle = (key: "coordinates" | "animation" | "highlights", label: string) => (
    <label className="flex items-center gap-2 text-sm">
      <input type="checkbox" checked={settings[key]} onChange={(e) => update({ [key]: e.target.checked })} className="size-4 accent-[var(--brand)]" />
      {label}
    </label>
  );

  return (
    <div className="grid gap-4 rounded-xl border border-line bg-surface p-4">
      <fieldset>
        <legend className="mb-2 text-sm font-semibold">Board</legend>
        <div className="flex flex-wrap gap-2">
          {BOARD_THEMES.map((t) => (
            <button
              key={t.id}
              type="button"
              onClick={() => update({ boardTheme: t.id })}
              aria-pressed={settings.boardTheme === t.id}
              aria-label={t.label}
              title={t.label}
              className={`size-10 overflow-hidden rounded-md border-2 ${settings.boardTheme === t.id ? "border-brand" : "border-line"}`}
              style={{
                background: `conic-gradient(${t.dark} 90deg, ${t.light} 0 180deg, ${t.dark} 0 270deg, ${t.light} 0)`,
                backgroundSize: "50% 50%",
              }}
            />
          ))}
        </div>
      </fieldset>
      <fieldset>
        <legend className="mb-2 text-sm font-semibold">Pieces</legend>
        <div className="flex flex-wrap gap-2">
          {PIECE_SETS.map((p) => (
            <button
              key={p.id}
              type="button"
              onClick={() => update({ pieceSet: p.id })}
              aria-pressed={settings.pieceSet === p.id}
              aria-label={p.label}
              title={p.label}
              className={`grid size-12 place-items-center rounded-md border-2 bg-surface-2 ${settings.pieceSet === p.id ? "border-brand" : "border-line"}`}
            >
              {/* eslint-disable-next-line @next/next/no-img-element -- tiny local SVGs */}
              <img src={`/pieces/${p.id}/wN.svg`} alt="" className="size-9" />
            </button>
          ))}
        </div>
      </fieldset>
      <div className="grid gap-2">
        {toggle("coordinates", "Show coordinates")}
        {toggle("highlights", "Highlight moves and legal squares")}
        {toggle("animation", "Animate pieces")}
      </div>
    </div>
  );
}
