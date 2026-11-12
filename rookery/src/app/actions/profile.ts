"use server";

import { z } from "zod";
import { revalidatePath } from "next/cache";
import { createClient } from "@/lib/supabase/server";
import { getCurrentProfile } from "@/lib/auth";
import { profileSchema } from "@/lib/validation";

export type ProfileFormState =
  | { ok?: boolean; error?: string; fieldErrors?: Record<string, string[] | undefined> }
  | undefined;

export async function updateProfile(_prev: ProfileFormState, formData: FormData): Promise<ProfileFormState> {
  const me = await getCurrentProfile();
  if (!me) return { error: "You need to be logged in." };

  const parsed = profileSchema.safeParse({
    display_name: formData.get("display_name") ?? "",
    bio: formData.get("bio") ?? "",
    country: formData.get("country") ?? "",
  });
  if (!parsed.success) return { fieldErrors: z.flattenError(parsed.error).fieldErrors };

  const supabase = await createClient();
  const { error } = await supabase
    .from("profiles")
    .update({
      display_name: parsed.data.display_name || null,
      bio: parsed.data.bio,
      country: parsed.data.country || null,
    })
    .eq("id", me.id);
  if (error) return { error: "Could not save. Please try again." };

  revalidatePath(`/u/${me.username}`);
  return { ok: true };
}
