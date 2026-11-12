"use server";

import { z } from "zod";
import { redirect } from "next/navigation";
import { revalidatePath } from "next/cache";
import { createClient } from "@/lib/supabase/server";
import { loginSchema, signupSchema, usernameToEmail } from "@/lib/validation";

export type FormState = { error?: string; fieldErrors?: Record<string, string[] | undefined> } | undefined;

export async function signup(_prev: FormState, formData: FormData): Promise<FormState> {
  const parsed = signupSchema.safeParse({
    username: formData.get("username"),
    password: formData.get("password"),
  });
  if (!parsed.success) return { fieldErrors: z.flattenError(parsed.error).fieldErrors };
  const { username, password } = parsed.data;

  const supabase = await createClient();

  const { data: taken } = await supabase.from("profiles").select("id").eq("username", username).maybeSingle();
  if (taken) return { fieldErrors: { username: ["That username is taken."] } };

  const { data, error } = await supabase.auth.signUp({
    email: usernameToEmail(username),
    password,
    options: { data: { username } },
  });

  if (error) {
    if (error.code === "over_email_send_rate_limit" || error.code === "email_address_invalid") {
      return { error: "Sign-ups are not switched on yet. Ask the site owner to turn off email confirmation." };
    }
    if (error.code === "user_already_exists") return { fieldErrors: { username: ["That username is taken."] } };
    return { error: "Could not create the account. Please try again." };
  }
  if (!data.session) {
    return { error: "Sign-ups are not switched on yet. Ask the site owner to turn off email confirmation." };
  }

  revalidatePath("/", "layout");
  redirect("/");
}

export async function login(_prev: FormState, formData: FormData): Promise<FormState> {
  const parsed = loginSchema.safeParse({
    username: formData.get("username"),
    password: formData.get("password"),
  });
  if (!parsed.success) return { fieldErrors: z.flattenError(parsed.error).fieldErrors };

  const supabase = await createClient();
  const { error } = await supabase.auth.signInWithPassword({
    email: usernameToEmail(parsed.data.username),
    password: parsed.data.password,
  });
  if (error) return { error: "Wrong username or password." };

  revalidatePath("/", "layout");
  redirect("/");
}

export async function logout() {
  const supabase = await createClient();
  await supabase.auth.signOut();
  revalidatePath("/", "layout");
  redirect("/");
}
