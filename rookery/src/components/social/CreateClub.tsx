"use client";

import { useRouter } from "next/navigation";
import { useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { CLUB_SLUG_RE, friendlyError, slugify, ui } from "@/lib/social";

export function CreateClub({ viewerId }: { viewerId: string }) {
  const router = useRouter();
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [open, setOpen] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const submit = async (e: React.FormEvent) => {
    e.preventDefault();
    const slug = slugify(name);
    if (!CLUB_SLUG_RE.test(slug)) return setError("Pick a name with at least 3 letters or numbers.");
    setBusy(true);
    setError(null);
    const { error: err } = await createClient().from("clubs").insert({ slug, name: name.trim(), description: description.trim(), owner_id: viewerId, open });
    setBusy(false);
    if (err) return setError(err.code === "23505" ? "A club with a similar name already exists." : friendlyError(err));
    router.push(`/clubs/${slug}`);
  };

  return (
    <form onSubmit={submit} className={`${ui.card} grid max-w-xl gap-3`}>
      <h2 className="text-xl font-bold">Start a club</h2>
      {error && <p role="alert" className={ui.err}>{error}</p>}
      <label className="grid gap-1.5 text-sm font-medium">Name
        <input className={ui.field} value={name} onChange={(e) => setName(e.target.value)} minLength={3} maxLength={40} required />
      </label>
      <label className="grid gap-1.5 text-sm font-medium">Description
        <textarea className={ui.field} rows={3} maxLength={400} value={description} onChange={(e) => setDescription(e.target.value)} />
      </label>
      <label className="flex items-center gap-2 text-sm">
        <input type="checkbox" checked={open} onChange={(e) => setOpen(e.target.checked)} />
        Anyone can join (untick to approve each request)
      </label>
      <button disabled={busy || name.trim().length < 3} className={`${ui.primary} justify-self-start`}>Create club</button>
    </form>
  );
}
