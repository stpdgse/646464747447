import { describe, expect, it } from "vitest";
import { loginSchema, passwordSchema, profileSchema, signupSchema, usernameSchema, usernameToEmail } from "./validation";

describe("usernameSchema", () => {
  it.each(["abc", "Magnus_C", "a1_b2", "x".repeat(20)])("accepts %s", (u) => {
    expect(usernameSchema.safeParse(u).success).toBe(true);
  });
  it.each(["ab", "1abc", "_abc", "has space", "emoji♞", "x".repeat(21), "dash-ed", ""])("rejects %j", (u) => {
    expect(usernameSchema.safeParse(u).success).toBe(false);
  });
  it("trims surrounding whitespace", () => {
    expect(usernameSchema.parse("  Hikaru  ")).toBe("Hikaru");
  });
});

describe("passwordSchema", () => {
  it("requires 8-72 characters", () => {
    expect(passwordSchema.safeParse("1234567").success).toBe(false);
    expect(passwordSchema.safeParse("12345678").success).toBe(true);
    expect(passwordSchema.safeParse("x".repeat(72)).success).toBe(true);
    expect(passwordSchema.safeParse("x".repeat(73)).success).toBe(false);
  });
});

describe("signup/login schemas", () => {
  it("validates signup", () => {
    expect(signupSchema.safeParse({ username: "Bobby", password: "password1" }).success).toBe(true);
    expect(signupSchema.safeParse({ username: "B", password: "password1" }).success).toBe(false);
  });
  it("login only needs non-empty fields", () => {
    expect(loginSchema.safeParse({ username: "x", password: "y" }).success).toBe(true);
    expect(loginSchema.safeParse({ username: "", password: "y" }).success).toBe(false);
  });
});

describe("profileSchema", () => {
  it("normalizes country to upper case and allows empty", () => {
    expect(profileSchema.parse({ bio: "", country: "de" }).country).toBe("DE");
    expect(profileSchema.parse({ bio: "", country: "" }).country).toBe("");
    expect(profileSchema.safeParse({ bio: "", country: "DEU" }).success).toBe(false);
  });
  it("caps bio at 400 characters", () => {
    expect(profileSchema.safeParse({ bio: "x".repeat(401), country: "" }).success).toBe(false);
  });
});

describe("usernameToEmail", () => {
  it("is case-insensitive so logins are too", () => {
    expect(usernameToEmail("MagnusC")).toBe(usernameToEmail("magnusc"));
    expect(usernameToEmail("Bobby")).toBe("bobby@rookery.example");
  });
});
