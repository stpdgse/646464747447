/** Shared helpers for friends, chat, clubs, forum and moderation. */

export const ONLINE_MS = 3 * 60_000;

export const isOnline = (lastSeen: string | null | undefined, now: number): boolean =>
  !!lastSeen && now - new Date(lastSeen).getTime() < ONLINE_MS;

export const REPORT_REASONS = [
  { id: "cheating", label: "Cheating" },
  { id: "abuse", label: "Abuse or harassment" },
  { id: "spam", label: "Spam" },
  { id: "inappropriate", label: "Inappropriate" },
  { id: "other", label: "Something else" },
] as const;
export type ReportReason = (typeof REPORT_REASONS)[number]["id"];
export type ReportKind = "user" | "game" | "game_message" | "club_message" | "forum_post" | "forum_thread";

/** Turn a Postgres / PostgREST error into something a student can read. */
export function friendlyError(e: { message?: string; code?: string } | null | undefined): string {
  const m = e?.message ?? "";
  if (m.includes("slow_down")) return "You're going too fast. Wait a few seconds and try again.";
  if (m.includes("too_many_clubs")) return "You can run up to 3 clubs.";
  if (m.includes("duplicate key") || e?.code === "23505") return "That already exists.";
  if (m.includes("row-level security") || m.includes("permission denied") || e?.code === "42501")
    return "You can't do that right now. You might be muted, blocked, or not allowed here.";
  if (m.includes("violates check constraint")) return "That doesn't fit the rules (check the length).";
  if (m.includes("forbidden_target")) return "You can't take action against that account.";
  if (m.includes("forbidden")) return "Only moderators can do that.";
  return "Something went wrong. Please try again.";
}

export const timeAgo = (iso: string, now: number): string => {
  const s = Math.max(0, Math.round((now - new Date(iso).getTime()) / 1000));
  if (s < 60) return "just now";
  if (s < 3600) return `${Math.floor(s / 60)} min ago`;
  if (s < 86400) return `${Math.floor(s / 3600)} h ago`;
  return `${Math.floor(s / 86400)} d ago`;
};

export const fmtDate = (iso: string): string => new Date(iso).toLocaleString("en", { dateStyle: "medium", timeStyle: "short" });

export const USERNAME_RE = /^[A-Za-z][A-Za-z0-9_]{2,19}$/;
export const CLUB_SLUG_RE = /^[a-z0-9][a-z0-9-]{2,29}$/;
export const slugify = (name: string): string =>
  name.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-+|-+$/g, "").slice(0, 30);

export const ui = {
  btn: "rounded-md border border-line bg-surface px-3 py-1.5 text-sm font-medium hover:bg-surface-2 disabled:opacity-40",
  primary: "rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-50",
  danger: "rounded-md border border-danger/50 px-3 py-1.5 text-sm font-medium text-danger hover:bg-danger/10 disabled:opacity-40",
  field: "w-full min-w-0 rounded-md border border-line bg-bg px-3 py-2 text-sm",
  card: "rounded-xl border border-line bg-surface p-5",
  err: "rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger",
  info: "rounded-md border border-line bg-surface px-3 py-2 text-sm text-dim",
} as const;

/** True while a mute/ban timestamp is still in the future. Reads the clock, so keep it out of render paths that must stay pure. */
export const isFuture = (iso: string | null | undefined): boolean => !!iso && new Date(iso).getTime() > Date.now();

/** Milliseconds since an ISO timestamp. Reads the clock, so call it from a function, not directly in render. */
export const ageMs = (iso: string): number => Date.now() - new Date(iso).getTime();
