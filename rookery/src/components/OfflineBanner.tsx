"use client";

import { useSyncExternalStore } from "react";

const subscribe = (cb: () => void) => {
  window.addEventListener("online", cb);
  window.addEventListener("offline", cb);
  return () => {
    window.removeEventListener("online", cb);
    window.removeEventListener("offline", cb);
  };
};

/** Tells people when their connection drops, and what still works. */
export function OfflineBanner() {
  const online = useSyncExternalStore(subscribe, () => navigator.onLine, () => true);
  if (online) return null;
  return (
    <div role="alert" className="border-b border-warn/60 bg-warn/15 px-4 py-2 text-center text-sm font-semibold">
      You&apos;re offline. Games against the computer keep working and are saved; online games reconnect when you&apos;re back.
    </div>
  );
}
