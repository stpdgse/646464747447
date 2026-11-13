"use client";

import { useCallback, useEffect, useState } from "react";
import { createClient } from "@/lib/supabase/client";
import { isNewer, type GameRow } from "@/lib/live/types";

export type Connection = "connecting" | "realtime" | "polling";

/**
 * Keeps one game row fresh. Supabase Realtime pushes changes instantly; if the connection cannot be made (a school
 * filter blocking WebSockets) or drops, the hook falls back to polling every 1.5s so games still work.
 * `offset` is (database clock - this device's clock) in ms, so clocks are right even if the device clock is wrong.
 */
export function useLiveGame(initial: GameRow) {
  const [row, setRow] = useState(initial);
  const [offset, setOffset] = useState(0);
  const [connection, setConnection] = useState<Connection>("connecting");
  const id = initial.id;

  const merge = useCallback((next: GameRow) => setRow((prev) => (isNewer(prev, next) ? next : prev)), []);

  useEffect(() => {
    const supabase = createClient();
    let dead = false;

    const sent = Date.now();
    supabase.rpc("server_time").then(({ data }) => {
      if (dead || !data) return;
      const rtt = Date.now() - sent;
      setOffset(Date.parse(data as string) + rtt / 2 - Date.now());
    });

    const channel = supabase
      .channel(`game-${id}`)
      .on("postgres_changes", { event: "UPDATE", schema: "public", table: "games", filter: `id=eq.${id}` }, (payload) => {
        if (!dead) merge(payload.new as GameRow);
      })
      .subscribe((status) => {
        if (dead) return;
        if (status === "SUBSCRIBED") setConnection("realtime");
        else if (status === "CHANNEL_ERROR" || status === "TIMED_OUT" || status === "CLOSED") setConnection("polling");
      });

    // If realtime has not connected after a few seconds, assume it is blocked.
    const giveUp = setTimeout(() => !dead && setConnection((c) => (c === "connecting" ? "polling" : c)), 5000);

    return () => {
      dead = true;
      clearTimeout(giveUp);
      supabase.removeChannel(channel);
    };
  }, [id, merge]);

  useEffect(() => {
    if (row.status !== "started") return;
    const supabase = createClient();
    let dead = false;
    const poll = async () => {
      const { data } = await supabase.from("games").select("*").eq("id", id).maybeSingle();
      if (data && !dead) merge(data as GameRow);
    };
    const every = connection === "realtime" ? 15_000 : 1_500;
    const timer = setInterval(() => document.visibilityState === "visible" && poll(), every);
    const onVisible = () => document.visibilityState === "visible" && poll();
    document.addEventListener("visibilitychange", onVisible);
    window.addEventListener("online", poll);
    window.addEventListener("focus", onVisible);
    return () => {
      dead = true;
      clearInterval(timer);
      document.removeEventListener("visibilitychange", onVisible);
      window.removeEventListener("online", poll);
      window.removeEventListener("focus", onVisible);
    };
  }, [connection, id, merge, row.status]);

  return { row, merge, offset, connection };
}
