import { readFileSync } from "node:fs";
import path from "node:path";
import { describe, expect, it } from "vitest";
import { ACHIEVEMENTS, CATEGORIES, achievementOf } from "./achievements";

const sql = readFileSync(path.resolve(__dirname, "../../supabase/migrations/20261112000020_rude_challenge.sql"), "utf8");
// The badge checker is redefined by later migrations, so read the newest definition.
const fnSql = sql.slice(sql.indexOf("create or replace function public.check_achievements()"));
const sqlKeys = [...fnSql.matchAll(/select '([a-z0-9_]+)'/g)].map((m) => m[1]);

describe("achievement catalogue", () => {
  it("has unique keys, names and a known category for every badge", () => {
    expect(new Set(ACHIEVEMENTS.map((a) => a.key)).size).toBe(ACHIEVEMENTS.length);
    expect(new Set(ACHIEVEMENTS.map((a) => a.name)).size).toBe(ACHIEVEMENTS.length);
    for (const a of ACHIEVEMENTS) {
      expect(CATEGORIES).toContain(a.category);
      expect(a.desc.length).toBeGreaterThan(10);
      expect(a.key).toMatch(/^[a-z0-9_]{2,40}$/);
    }
    expect(achievementOf("first_win")?.name).toBe("First blood");
    expect(achievementOf("nope")).toBeUndefined();
  });
  it("matches the database function exactly, so no badge can be earned without a description or the reverse", () => {
    expect([...sqlKeys].sort()).toEqual(ACHIEVEMENTS.map((a) => a.key).sort());
  });
});
