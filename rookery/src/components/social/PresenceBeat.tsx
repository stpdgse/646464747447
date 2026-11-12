"use client";

import { useEffect } from "react";
import { createClient } from "@/lib/supabase/client";

/** Tells the database "I'm here" once a minute while the tab is visible. Friends see a green dot. (The database stamps the time.) */
export function PresenceBeat({ userId }: { userId: string }) {
  useEffect(() => {
    const supabase = createClient();
    const beat = () => {
      if (document.visibilityState !== "visible") return;
      supabase.from("presence").upsert({ user_id: userId, last_seen_at: new Date().toISOString() }, { onConflict: "user_id" }).then(() => {});
    };
    queueMicrotask(beat);
    const timer = setInterval(beat, 60_000);
    document.addEventListener("visibilitychange", beat);
    return () => {
      clearInterval(timer);
      document.removeEventListener("visibilitychange", beat);
    };
  }, [userId]);
  return null;
}
