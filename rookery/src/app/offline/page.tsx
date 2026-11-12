import type { Metadata } from "next";

export const metadata: Metadata = { title: "Offline" };

export default function OfflinePage() {
  return (
    <div className="mx-auto grid max-w-md gap-4 py-16 text-center">
      <p aria-hidden className="text-6xl">♜</p>
      <h1 className="text-3xl font-extrabold tracking-tight">You&apos;re offline</h1>
      <p className="text-dim">
        The site needs a connection to load games and friends. Your device will reconnect on its own. You can still play the
        computer once the page has loaded.
      </p>
    </div>
  );
}
