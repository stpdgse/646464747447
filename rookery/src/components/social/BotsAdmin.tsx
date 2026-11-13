"use client";

import { useCallback, useEffect, useMemo, useState } from "react";
import { fetchCustomBots } from "@/components/computer/useBots";
import { BOTS, mergeBots, smartnessToPlay, suggestRating, SMARTNESS_MAX, type Bot, type CustomBotRow, type Style } from "@/lib/engine/bots";
import { pictureToDataUrl } from "@/lib/image";
import { slugify, ui } from "@/lib/social";
import { createClient } from "@/lib/supabase/client";

const STYLES: { id: Style; label: string; hint: string }[] = [
  { id: "balanced", label: "Balanced", hint: "No preference" },
  { id: "aggressive", label: "Aggressive", hint: "Captures, checks and promotions" },
  { id: "solid", label: "Solid", hint: "Quiet, safe moves" },
  { id: "greedy", label: "Greedy", hint: "Grabs material whenever it can" },
  { id: "checker", label: "Checker", hint: "Loves giving check" },
  { id: "chaotic", label: "Chaotic", hint: "Random flair" },
];

const levelName = (s: number) => (s === 0 ? "Random mover" : s <= 5 ? "Beginner" : s <= 10 ? "Casual" : s <= 13 ? "Club player" : s <= 18 ? "Strong engine" : s <= 22 ? "Master engine" : "Maximum");

type Form = {
  id: string; isNew: boolean; builtin: boolean; name: string; avatar: string; avatar_image: string | null; basePicture: string | null; rating: number; blurb: string; greeting: string;
  quip_win: string; quip_lose: string; quip_draw: string; useSmartness: boolean; smartness: number; style: Style; style_bonus: number; styleOn: boolean;
  blunderOn: boolean; blunder_pct: number; mateOn: boolean; mate_sight_pct: number; thinkOn: boolean; thinking_ms: number; special: boolean; hidden: boolean;
};

const blank = (): Form => ({
  id: "", isNew: true, builtin: false, name: "", avatar: "🤖", avatar_image: null, basePicture: null, rating: 1000, blurb: "", greeting: "", quip_win: "", quip_lose: "", quip_draw: "",
  useSmartness: true, smartness: 10, style: "balanced", style_bonus: 40, styleOn: false, blunderOn: false, blunder_pct: 5, mateOn: false, mate_sight_pct: 90, thinkOn: false, thinking_ms: 500,
  special: false, hidden: false,
});

/** Start the form from what the bot is right now (built-in values, plus any admin edits). */
function formFrom(bot: Bot, row: CustomBotRow | undefined, builtin: boolean): Form {
  return {
    id: bot.id, isNew: false, builtin, name: bot.name, avatar: bot.avatar, avatar_image: bot.avatarImage?.startsWith("data:") ? bot.avatarImage : null, basePicture: bot.avatarImage && !bot.avatarImage.startsWith("data:") ? bot.avatarImage : null, rating: bot.rating, blurb: bot.blurb, greeting: bot.greeting,
    quip_win: bot.quips.win, quip_lose: bot.quips.lose, quip_draw: bot.quips.draw,
    useSmartness: !builtin || (row?.smartness ?? null) !== null, smartness: row?.smartness ?? 10,
    style: bot.human.style, style_bonus: bot.human.styleBonus, styleOn: !builtin || !!row?.style, blunderOn: !builtin || row?.blunder_pct != null, blunder_pct: Math.round(bot.human.blunderRate * 100),
    mateOn: !builtin || row?.mate_sight_pct != null, mate_sight_pct: Math.round(bot.human.mateSight * 100), thinkOn: !builtin || row?.thinking_ms != null, thinking_ms: bot.engine.movetime ?? 500,
    special: !!bot.special, hidden: row?.hidden ?? false,
  };
}

/** The database row for a form. For a built-in bot the habits left switched off stay null so the original value is kept. */
function rowFrom(f: Form): Omit<CustomBotRow, never> & { id: string } {
  return {
    id: f.id, name: f.name.trim(), avatar: f.avatar.trim() || "🤖", avatar_image: f.avatar_image, rating: Math.round(f.rating), blurb: f.blurb.trim(), greeting: f.greeting.trim(),
    quip_win: f.quip_win.trim(), quip_lose: f.quip_lose.trim(), quip_draw: f.quip_draw.trim(),
    smartness: f.useSmartness ? Math.round(f.smartness) : null,
    style: f.styleOn ? f.style : null, style_bonus: f.styleOn ? Math.round(f.style_bonus) : null,
    blunder_pct: f.blunderOn ? Math.round(f.blunder_pct) : null, mate_sight_pct: f.mateOn ? Math.round(f.mate_sight_pct) : null, thinking_ms: f.thinkOn ? Math.round(f.thinking_ms) : null,
    special: f.special, hidden: f.hidden,
  };
}

const lbl = "grid gap-1.5 text-sm font-medium";

export function BotsAdmin() {
  const [rows, setRows] = useState<CustomBotRow[] | null>(null);
  const [form, setForm] = useState<Form | null>(null);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);
  const [busy, setBusy] = useState(false);

  const load = useCallback(async () => {
    const r = await fetchCustomBots();
    if (r) setRows(r);
  }, []);
  useEffect(() => {
    queueMicrotask(load);
  }, [load]);

  const builtinIds = useMemo(() => new Set(BOTS.map((b) => b.id)), []);
  // Every bot, including hidden ones, with its current (edited) values.
  const list = useMemo(() => {
    if (!rows) return [];
    const visible = mergeBots(BOTS, rows.map((r) => ({ ...r, hidden: false })));
    return visible.map((b) => ({ bot: b, row: rows.find((r) => r.id === b.id), builtin: builtinIds.has(b.id) }));
  }, [rows, builtinIds]);

  const save = async () => {
    if (!form) return;
    setMsg(null);
    const f = { ...form };
    if (!f.name.trim()) return setMsg({ ok: false, text: "Give the bot a name." });
    if (f.isNew) {
      const slug = f.id || slugify(f.name);
      if (!/^[a-z0-9][a-z0-9-]{1,29}$/.test(slug)) return setMsg({ ok: false, text: "The name needs at least 2 letters or numbers." });
      if (builtinIds.has(slug) || rows?.some((r) => r.id === slug)) return setMsg({ ok: false, text: "A bot with that id already exists. Change the name a little." });
      f.id = slug;
    }
    if (!f.useSmartness && !f.builtin) return setMsg({ ok: false, text: "A new bot needs a smartness level." });
    setBusy(true);
    const supabase = createClient();
    const row = rowFrom(f);
    const exists = rows?.some((r) => r.id === f.id);
    const { error } = exists ? await supabase.from("custom_bots").update(row).eq("id", f.id) : await supabase.from("custom_bots").insert(row);
    setBusy(false);
    if (error) return setMsg({ ok: false, text: error.message.includes("check constraint") ? "One of the values is out of range." : error.message.includes("row-level security") ? "Only admins can do that." : "Could not save. Try again." });
    setMsg({ ok: true, text: `Saved ${row.name}. Everyone sees it the next time they open the bot list.` });
    setForm(null);
    load();
  };

  const remove = async (id: string, builtin: boolean) => {
    if (!confirm(builtin ? "Reset this bot to its original settings?" : "Delete this bot for good?")) return;
    const { error } = await createClient().from("custom_bots").delete().eq("id", id);
    setMsg(error ? { ok: false, text: "Could not remove it." } : { ok: true, text: builtin ? "Bot reset to its original settings." : "Bot deleted." });
    load();
  };

  const toggleHidden = async (id: string, builtin: boolean, bot: Bot, row: CustomBotRow | undefined, hidden: boolean) => {
    const supabase = createClient();
    const { error } = row
      ? await supabase.from("custom_bots").update({ hidden }).eq("id", id)
      : await supabase.from("custom_bots").insert({ ...rowFrom({ ...formFrom(bot, undefined, builtin), hidden, useSmartness: false, styleOn: false, blunderOn: false, mateOn: false, thinkOn: false }) });
    if (error) setMsg({ ok: false, text: "Could not change that." });
    load();
  };

  const set = <K extends keyof Form>(k: K, v: Form[K]) => setForm((f) => (f ? { ...f, [k]: v } : f));

  if (form) {
    const preview = smartnessToPlay(form.smartness);
    return (
      <div className="grid min-w-0 gap-5">
        <div className="flex items-center gap-3">
          <button className={ui.btn} onClick={() => { setForm(null); setMsg(null); }}>← All bots</button>
          <h2 className="text-xl font-bold">{form.isNew ? "New bot" : `Edit ${form.name || form.id}`}{form.builtin && <span className="ml-2 text-sm font-normal text-dim">(built-in)</span>}</h2>
        </div>
        {msg && <p role={msg.ok ? "status" : "alert"} className={msg.ok ? ui.info : ui.err}>{msg.text}</p>}

        <section className={`${ui.card} grid gap-4`} aria-label="Looks and words">
          <h3 className="font-bold">Looks and words</h3>
          <div className="flex flex-wrap items-center gap-4">
            {form.avatar_image || form.basePicture ? <img src={(form.avatar_image ?? form.basePicture)!} alt="" width={64} height={64} className="size-16 rounded-lg" /> : <span className="grid size-16 place-items-center rounded-lg bg-surface-2 text-4xl" aria-hidden>{form.avatar || "🤖"}</span>}
            <div className="grid gap-2">
              <label className="text-sm font-medium">Picture
                <input type="file" accept="image/png,image/jpeg,image/webp,image/gif" className="mt-1 block text-sm"
                  onChange={async (e) => {
                    const file = e.target.files?.[0];
                    if (!file) return;
                    try { set("avatar_image", await pictureToDataUrl(file)); setMsg(null); } catch (err) { setMsg({ ok: false, text: (err as Error).message }); }
                    e.target.value = "";
                  }} />
              </label>
              {form.avatar_image && <button type="button" className="justify-self-start text-xs text-dim underline" onClick={() => set("avatar_image", null)}>Remove picture (use the emoji)</button>}
            </div>
            <label className={`${lbl} w-28`}>Emoji
              <input className={ui.field} value={form.avatar} maxLength={8} onChange={(e) => set("avatar", e.target.value)} />
            </label>
          </div>
          <label className={lbl}>Name
            <input className={ui.field} value={form.name} maxLength={40} onChange={(e) => set("name", e.target.value)} />
          </label>
          <label className={lbl}>Description
            <input className={ui.field} value={form.blurb} maxLength={200} onChange={(e) => set("blurb", e.target.value)} />
          </label>
          <label className={lbl}>Says when the game starts
            <input className={ui.field} value={form.greeting} maxLength={160} onChange={(e) => set("greeting", e.target.value)} />
          </label>
          <div className="grid gap-3 sm:grid-cols-3">
            <label className={lbl}>…when the bot wins<input className={ui.field} value={form.quip_win} maxLength={160} onChange={(e) => set("quip_win", e.target.value)} /></label>
            <label className={lbl}>…when the bot loses<input className={ui.field} value={form.quip_lose} maxLength={160} onChange={(e) => set("quip_lose", e.target.value)} /></label>
            <label className={lbl}>…on a draw<input className={ui.field} value={form.quip_draw} maxLength={160} onChange={(e) => set("quip_draw", e.target.value)} /></label>
          </div>
          <div className="flex flex-wrap gap-6">
            <label className="flex items-center gap-2 text-sm"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.special} onChange={(e) => set("special", e.target.checked)} /> Show the &ldquo;Special&rdquo; badge</label>
            <label className="flex items-center gap-2 text-sm"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.hidden} onChange={(e) => set("hidden", e.target.checked)} /> Hide from the bot list</label>
          </div>
        </section>

        <section className={`${ui.card} grid gap-4`} aria-label="Strength">
          <h3 className="font-bold">Strength</h3>
          <label className={lbl}>Rating shown to players (a label, tuned by feel)
            <span className="flex flex-wrap items-center gap-2">
              <input type="number" min={0} max={5000} className={`${ui.field} w-32`} value={form.rating} onChange={(e) => set("rating", Number(e.target.value))} />
              {form.useSmartness && <button type="button" className={ui.btn} onClick={() => set("rating", suggestRating(form.smartness))}>Suggest from smartness ({suggestRating(form.smartness)})</button>}
            </span>
          </label>
          {form.builtin && (
            <label className="flex items-center gap-2 text-sm">
              <input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.useSmartness} onChange={(e) => set("useSmartness", e.target.checked)} />
              Change how well this bot plays (otherwise it keeps its original strength)
            </label>
          )}
          {form.useSmartness && (
            <label className={lbl}>
              Smartness: <strong>{form.smartness}</strong> of {SMARTNESS_MAX} · {levelName(form.smartness)}
              <input type="range" min={0} max={SMARTNESS_MAX} value={form.smartness} onChange={(e) => set("smartness", Number(e.target.value))} className="accent-[var(--brand)]" />
              <span className="text-xs font-normal text-dim">
                0 plays almost random moves and never sees a mate. 24 is the full engine.
                {preview.engine.elo ? ` Engine Elo ${preview.engine.elo}.` : ` Searches ${preview.engine.depth} move${preview.engine.depth === 1 ? "" : "s"} deep.`}
              </span>
            </label>
          )}
        </section>

        <section className={`${ui.card} grid gap-4`} aria-label="Playing habits">
          <h3 className="font-bold">Playing habits</h3>
          <p className="text-xs text-dim">{form.builtin ? "Tick a habit to change it; unticked habits keep their original value." : "Tick a habit to override what smartness picks."}</p>
          <div className="grid gap-2">
            <label className="flex items-center gap-2 text-sm font-medium"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.styleOn} onChange={(e) => set("styleOn", e.target.checked)} /> Style</label>
            {form.styleOn && (
              <div className="grid gap-2 pl-6">
                <select className={`${ui.field} max-w-xs`} value={form.style} onChange={(e) => set("style", e.target.value as Style)}>
                  {STYLES.map((s) => <option key={s.id} value={s.id}>{s.label}: {s.hint}</option>)}
                </select>
                <label className="text-xs text-dim">How strongly: {form.style_bonus}
                  <input type="range" min={0} max={300} value={form.style_bonus} onChange={(e) => set("style_bonus", Number(e.target.value))} className="block w-64 accent-[var(--brand)]" />
                </label>
              </div>
            )}
          </div>
          <div className="grid gap-2">
            <label className="flex items-center gap-2 text-sm font-medium"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.blunderOn} onChange={(e) => set("blunderOn", e.target.checked)} /> Blunders</label>
            {form.blunderOn && <label className="pl-6 text-xs text-dim">Plays a deliberately bad move {form.blunder_pct}% of the time
              <input type="range" min={0} max={90} value={form.blunder_pct} onChange={(e) => set("blunder_pct", Number(e.target.value))} className="block w-64 accent-[var(--brand)]" /></label>}
          </div>
          <div className="grid gap-2">
            <label className="flex items-center gap-2 text-sm font-medium"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.mateOn} onChange={(e) => set("mateOn", e.target.checked)} /> Sees checkmates</label>
            {form.mateOn && <label className="pl-6 text-xs text-dim">Notices a forced mate {form.mate_sight_pct}% of the time
              <input type="range" min={0} max={100} value={form.mate_sight_pct} onChange={(e) => set("mate_sight_pct", Number(e.target.value))} className="block w-64 accent-[var(--brand)]" /></label>}
          </div>
          <div className="grid gap-2">
            <label className="flex items-center gap-2 text-sm font-medium"><input type="checkbox" className="size-4 accent-[var(--brand)]" checked={form.thinkOn} onChange={(e) => set("thinkOn", e.target.checked)} /> Thinking time per move</label>
            {form.thinkOn && <label className="pl-6 text-xs text-dim">{form.thinking_ms} ms
              <input type="range" min={50} max={5000} step={50} value={form.thinking_ms} onChange={(e) => set("thinking_ms", Number(e.target.value))} className="block w-64 accent-[var(--brand)]" /></label>}
          </div>
        </section>

        <div className="flex flex-wrap gap-3">
          <button disabled={busy} onClick={save} className={ui.primary}>{form.isNew ? "Create bot" : "Save changes"}</button>
          <button className={ui.btn} onClick={() => { setForm(null); setMsg(null); }}>Cancel</button>
        </div>
      </div>
    );
  }

  return (
    <div className="grid min-w-0 gap-4">
      <div className="flex flex-wrap items-center gap-3">
        <button className={ui.primary} onClick={() => { setForm(blank()); setMsg(null); }}>+ New bot</button>
        <p className="text-sm text-dim">Add bots or change any existing one: picture, name, description, rating, smartness and habits.</p>
      </div>
      {msg && <p role={msg.ok ? "status" : "alert"} className={msg.ok ? ui.info : ui.err}>{msg.text}</p>}
      {rows === null ? <div className="h-24 animate-pulse rounded-xl bg-surface" /> : (
        <ul className="grid gap-2">
          {list.map(({ bot, row, builtin }) => (
            <li key={bot.id} className={`${ui.card} flex flex-wrap items-center gap-3 py-3 ${row?.hidden ? "opacity-60" : ""}`}>
              {bot.avatarImage ? <img src={bot.avatarImage} alt="" width={40} height={40} className="size-10 rounded-md" /> : <span aria-hidden className="grid size-10 place-items-center rounded-md bg-surface-2 text-2xl">{bot.avatar}</span>}
              <span className="min-w-0 flex-1">
                <span className="font-bold">{bot.name}</span>
                <span className="ml-2 text-xs font-semibold text-brand">≈ {bot.rating}</span>
                <span className="ml-2 text-xs text-dim">{builtin ? (row ? "built-in, edited" : "built-in") : "custom"}{row?.hidden ? " · hidden" : ""}{bot.special ? " · special" : ""}</span>
                <span className="block truncate text-xs text-dim">{bot.blurb}</span>
              </span>
              <span className="flex flex-wrap gap-2">
                <a href={`/play/computer?bot=${bot.id}`} className={ui.btn}>Try it</a>
                <button className={ui.btn} onClick={() => { setForm(formFrom(bot, row, builtin)); setMsg(null); }}>Edit</button>
                <button className={ui.btn} onClick={() => toggleHidden(bot.id, builtin, bot, row, true)}>Hide</button>
                {row && <button className={ui.danger} onClick={() => remove(bot.id, builtin)}>{builtin ? "Reset" : "Delete"}</button>}
              </span>
            </li>
          ))}
          {rows.filter((r) => r.hidden).map((r) => {
            const base = BOTS.find((b) => b.id === r.id);
            return (
              <li key={`h-${r.id}`} className={`${ui.card} flex flex-wrap items-center gap-3 py-3 opacity-70`}>
                <span className="min-w-0 flex-1"><span className="font-bold">{r.name}</span><span className="ml-2 text-xs text-dim">hidden{base ? " built-in" : " custom"}</span></span>
                <span className="flex gap-2">
                  <button className={ui.btn} onClick={async () => {
                    // A built-in bot that was only hidden has nothing else to keep: drop the row so it is plainly built-in again.
                    const untouched = !!base && r.smartness === null && r.style === null && r.style_bonus === null && r.blunder_pct === null && r.mate_sight_pct === null && r.thinking_ms === null
                      && r.name === base.name && r.rating === base.rating && r.blurb === base.blurb && r.greeting === base.greeting && r.avatar === base.avatar && !r.avatar_image
                      && r.quip_win === base.quips.win && r.quip_lose === base.quips.lose && r.quip_draw === base.quips.draw && r.special === !!base.special;
                    const supabase = createClient();
                    if (untouched) await supabase.from("custom_bots").delete().eq("id", r.id);
                    else await supabase.from("custom_bots").update({ hidden: false }).eq("id", r.id);
                    load();
                  }}>Show again</button>
                  {!base && <button className={ui.danger} onClick={() => remove(r.id, false)}>Delete</button>}
                </span>
              </li>
            );
          })}
        </ul>
      )}
    </div>
  );
}
