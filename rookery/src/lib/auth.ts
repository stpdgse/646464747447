import "server-only";
import { cache } from "react";
import { redirect } from "next/navigation";
import { createClient } from "@/lib/supabase/server";

export type Profile = {
  id: string;
  username: string;
  display_name: string | null;
  bio: string;
  country: string | null;
  is_admin: boolean;
  is_moderator: boolean;
  muted_until: string | null;
  banned_until: string | null;
  settings: Record<string, unknown>;
  created_at: string;
};

/** Current signed-in profile, or null. Deduplicated per request. */
export const getCurrentProfile = cache(async (): Promise<Profile | null> => {
  const supabase = await createClient();
  const { data } = await supabase.auth.getClaims();
  const userId = data?.claims?.sub;
  if (!userId) return null;
  const { data: profile } = await supabase.from("profiles").select("*").eq("id", userId).single();
  return (profile as Profile | null) ?? null;
});

/** For pages that need a signed-in user. */
export async function requireProfile(): Promise<Profile> {
  const profile = await getCurrentProfile();
  if (!profile) redirect("/login");
  return profile;
}
