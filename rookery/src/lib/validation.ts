import { z } from "zod";

export const usernameSchema = z
  .string()
  .trim()
  .regex(
    /^[A-Za-z][A-Za-z0-9_]{2,19}$/,
    "3-20 characters: letters, numbers and underscores, starting with a letter.",
  );

export const passwordSchema = z
  .string()
  .min(8, "At least 8 characters.")
  .max(72, "At most 72 characters.");

export const signupSchema = z.object({ username: usernameSchema, password: passwordSchema });
export const loginSchema = z.object({
  username: z.string().trim().min(1, "Enter your username."),
  password: z.string().min(1, "Enter your password."),
});

export const profileSchema = z.object({
  display_name: z.string().trim().max(40, "At most 40 characters.").optional(),
  bio: z.string().trim().max(400, "At most 400 characters."),
  country: z
    .string()
    .trim()
    .toUpperCase()
    .regex(/^[A-Z]{2}$/, "Use a 2-letter country code, like DE or US.")
    .or(z.literal("")),
});

/**
 * Supabase Auth requires an email. Players sign in with a username, so we derive a stable,
 * non-routable address from it. The ".example" TLD is reserved and can never receive mail.
 */
export function usernameToEmail(username: string): string {
  return `${username.toLowerCase()}@rookery.example`;
}
