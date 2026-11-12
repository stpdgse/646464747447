import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { AdminPanel } from "@/components/social/AdminPanel";
import { getCurrentProfile } from "@/lib/auth";

export const metadata: Metadata = { title: "Moderation", robots: { index: false } };

export default function AdminPage({ searchParams }: PageProps<"/admin">) {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Moderation</h1>
      <Suspense fallback={<div className="h-48 animate-pulse rounded-xl bg-surface" />}>
        <Loader searchParams={searchParams} />
      </Suspense>
    </div>
  );
}

async function Loader({ searchParams }: { searchParams: PageProps<"/admin">["searchParams"] }) {
  const me = await getCurrentProfile();
  if (!me || !(me.is_admin || me.is_moderator)) notFound();
  const sp = await searchParams;
  const one = (k: string) => (typeof sp[k] === "string" ? (sp[k] as string) : "");
  const tab = (["reports", "fairplay", "users", "log"] as const).find((t) => t === one("tab")) ?? "reports";
  return <AdminPanel initialTab={tab} initialGame={one("game")} initialUser={one("user")} isAdmin={me.is_admin} />;
}
