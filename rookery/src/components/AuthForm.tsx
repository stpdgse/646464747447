"use client";

import Link from "next/link";
import { useActionState } from "react";
import type { FormState } from "@/app/actions/auth";

type Props = {
  mode: "login" | "signup";
  action: (prev: FormState, formData: FormData) => Promise<FormState>;
};

const inputCls =
  "w-full rounded-md border border-line bg-bg px-3 py-2 text-ink placeholder:text-dim focus:border-brand focus:outline-none";

export function AuthForm({ mode, action }: Props) {
  const [state, formAction, pending] = useActionState(action, undefined);
  const signup = mode === "signup";

  return (
    <form action={formAction} className="grid gap-4" noValidate>
      {state?.error && (
        <p role="alert" className="rounded-md border border-danger/50 bg-danger/10 px-3 py-2 text-sm text-danger">
          {state.error}
        </p>
      )}
      <div className="grid gap-1.5">
        <label htmlFor="username" className="text-sm font-medium">Username</label>
        <input
          id="username" name="username" autoComplete="username" required
          autoCapitalize="none" spellCheck={false} className={inputCls}
          aria-invalid={!!state?.fieldErrors?.username}
        />
        {state?.fieldErrors?.username?.map((m) => <p key={m} className="text-sm text-danger">{m}</p>)}
      </div>
      <div className="grid gap-1.5">
        <label htmlFor="password" className="text-sm font-medium">Password</label>
        <input
          id="password" name="password" type="password" required
          autoComplete={signup ? "new-password" : "current-password"} className={inputCls}
          aria-invalid={!!state?.fieldErrors?.password}
        />
        {state?.fieldErrors?.password?.map((m) => <p key={m} className="text-sm text-danger">{m}</p>)}
        {signup && <p className="text-xs text-dim">At least 8 characters. No email needed.</p>}
      </div>
      <button
        type="submit" disabled={pending}
        className="rounded-md bg-brand px-4 py-2.5 font-semibold text-brand-ink hover:bg-brand-hi disabled:opacity-60"
      >
        {pending ? "One moment…" : signup ? "Create account" : "Log in"}
      </button>
      <p className="text-center text-sm text-dim">
        {signup ? (
          <>Already have an account? <Link href="/login" className="text-brand hover:underline">Log in</Link></>
        ) : (
          <>New here? <Link href="/signup" className="text-brand hover:underline">Create an account</Link></>
        )}
      </p>
    </form>
  );
}
