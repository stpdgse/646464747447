"use client";

import { useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { REPORT_REASONS, friendlyError, ui, type ReportKind, type ReportReason } from "@/lib/social";

/** A small "Report" link that opens an inline form. Reports go to the moderators' queue. */
export function ReportButton({
  viewerId, kind, refId, targetUser, label = "Report", defaultReason = "abuse", className,
}: {
  viewerId: string | null; kind: ReportKind; refId: string; targetUser: string | null;
  label?: string; defaultReason?: ReportReason; className?: string;
}) {
  const [open, setOpen] = useState(false);
  const [reason, setReason] = useState<ReportReason>(defaultReason);
  const [details, setDetails] = useState("");
  const [busy, setBusy] = useState(false);
  const [msg, setMsg] = useState<{ ok: boolean; text: string } | null>(null);

  if (!viewerId) return null;

  const send = async () => {
    setBusy(true);
    const { error } = await createClient().from("reports").insert({
      reporter_id: viewerId, target_user: targetUser, kind, ref_id: refId, reason, details: details.trim(),
    });
    setBusy(false);
    if (error) {
      setMsg({ ok: false, text: error.code === "23505" ? "You already reported this. A moderator will look at it." : friendlyError(error) });
    } else {
      setMsg({ ok: true, text: "Thanks. A moderator will take a look." });
      setOpen(false);
    }
  };

  if (msg?.ok) return <span role="status" className="text-xs text-dim">{msg.text}</span>;

  return (
    <span className="inline-block">
      <button type="button" onClick={() => setOpen((o) => !o)} aria-expanded={open} className={className ?? "text-xs text-dim underline-offset-2 hover:text-ink hover:underline"}>
        {label}
      </button>
      {open && (
        <span className="mt-2 grid w-64 gap-2 rounded-lg border border-line bg-surface p-3 text-left shadow-xl">
          <label className="grid gap-1 text-xs font-medium">Reason
            <select className={ui.field} value={reason} onChange={(e) => setReason(e.target.value as ReportReason)}>
              {REPORT_REASONS.map((r) => <option key={r.id} value={r.id}>{r.label}</option>)}
            </select>
          </label>
          <label className="grid gap-1 text-xs font-medium">Details (optional)
            <textarea className={ui.field} rows={2} maxLength={500} value={details} onChange={(e) => setDetails(e.target.value)} />
          </label>
          {msg && <span role="alert" className="text-xs text-danger">{msg.text}</span>}
          <span className="flex gap-2">
            <button type="button" disabled={busy} onClick={send} className={ui.primary}>Send report</button>
            <button type="button" onClick={() => setOpen(false)} className={ui.btn}>Cancel</button>
          </span>
        </span>
      )}
    </span>
  );
}
