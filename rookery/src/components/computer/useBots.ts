"use client";

import { useEffect, useState } from "react";
import { BOTS, mergeBots, setBotRegistry, type Bot, type CustomBotRow } from "@/lib/engine/bots";
import { createClient } from "@/lib/supabase/client";

const CACHE_KEY = "rookery.custombots.v1";

function readCache(): CustomBotRow[] | null {
  try {
    const raw = window.localStorage.getItem(CACHE_KEY);
    const rows = raw ? JSON.parse(raw) : null;
    return Array.isArray(rows) ? (rows as CustomBotRow[]) : null;
  } catch {
    return null;
  }
}

/** Fetch the admins' bots again. Used by the admin editor after a save. */
export async function fetchCustomBots(): Promise<CustomBotRow[] | null> {
  const { data, error } = await createClient().from("custom_bots").select("*");
  if (error || !data) return null;
  try {
    window.localStorage.setItem(CACHE_KEY, JSON.stringify(data));
  } catch {
    /* ignore */
  }
  return data as CustomBotRow[];
}

/**
 * The bots on offer: the built-in roster plus whatever admins added or changed. Starts with the built-ins (so the server render
 * matches), switches to the last known list from this device straight away, then refreshes from the database. Offline keeps working.
 */
export function useBots(): Bot[] {
  const [bots, setBots] = useState<Bot[]>(BOTS);
  useEffect(() => {
    let dead = false;
    const apply = (rows: CustomBotRow[]) => {
      const merged = mergeBots(BOTS, rows);
      setBotRegistry(merged);
      if (!dead) setBots(merged);
    };
    queueMicrotask(() => {
      const cached = readCache();
      if (cached) apply(cached);
    });
    fetchCustomBots().then((rows) => rows && apply(rows)).catch(() => {});
    return () => {
      dead = true;
    };
  }, []);
  return bots;
}
