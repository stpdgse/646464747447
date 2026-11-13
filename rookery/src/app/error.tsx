"use client";

import { useEffect } from "react";

/** Last-resort screen if a page crashes. Games against the computer are saved, so reloading is safe. */
export default function ErrorPage({ error, reset }: { error: Error & { digest?: string }; reset: () => void }) {
  useEffect(() => {
    console.error(error);
  }, [error]);
  return (
    <div className="mx-auto grid max-w-md gap-4 py-16 text-center">
      <h1 className="text-3xl font-extrabold tracking-tight">Something went wrong</h1>
      <p className="text-dim">Your game is safe. Games against the computer are saved on this device, and online games keep running on the server.</p>
      <div className="flex justify-center gap-3">
        <button onClick={reset} className="rounded-md bg-brand px-4 py-2 text-sm font-semibold text-brand-ink hover:bg-brand-hi">Try again</button>
        <button onClick={() => window.location.reload()} className="rounded-md border border-line bg-surface px-4 py-2 text-sm font-medium hover:bg-surface-2">Reload the page</button>
      </div>
    </div>
  );
}
