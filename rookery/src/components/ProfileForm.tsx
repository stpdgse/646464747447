"use client";

import { useActionState } from "react";
import type { ProfileFormState } from "@/app/actions/profile";

const inputCls =
  "w-full rounded-md border border-line bg-bg px-3 py-2 text-ink focus:border-brand focus:outline-none";

export function ProfileForm({
  action,
  initial,
}: {
  action: (prev: ProfileFormState, formData: FormData) => Promise<ProfileFormState>;
  initial: { display_name: string; bio: string; country: string };
}) {
  const [state, formAction, pending] = useActionState(action, undefined);
  const err = (k: string) => state?.fieldErrors?.[k]?.map((m) => <p key={m} className="text-sm text-danger">{m}</p>);

  return (
    <form action={formAction} className="grid gap-4 rounded-xl border border-line bg-surface p-5">
      {state?.error && <p role="alert" className="text-sm text-danger">{state.error}</p>}
      {state?.ok && <p role="status" className="text-sm text-brand">Saved.</p>}
      <div className="grid gap-1.5">
        <label htmlFor="display_name" className="text-sm font-medium">Display name</label>
        <input id="display_name" name="display_name" defaultValue={initial.display_name} maxLength={40} className={inputCls} />
        {err("display_name")}
      </div>
      <div className="grid gap-1.5">
        <label htmlFor="country" className="text-sm font-medium">Country code</label>
        <input id="country" name="country" defaultValue={initial.country} maxLength={2} placeholder="DE" className={`${inputCls} w-24 uppercase`} />
        {err("country")}
      </div>
      <div className="grid gap-1.5">
        <label htmlFor="bio" className="text-sm font-medium">About you</label>
        <textarea id="bio" name="bio" defaultValue={initial.bio} maxLength={400} rows={4} className={inputCls} />
        {err("bio")}
      </div>
      <button disabled={pending} className="justify-self-start rounded-md bg-brand px-4 py-2 font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-60">
        {pending ? "Saving…" : "Save"}
      </button>
    </form>
  );
}
