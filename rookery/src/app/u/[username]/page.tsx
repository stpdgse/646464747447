import type { Metadata } from "next";
import { notFound } from "next/navigation";
import { Suspense } from "react";
import { createClient } from "@/lib/supabase/server";
import type { Profile } from "@/lib/auth";

export async function generateMetadata({ params }: PageProps<"/u/[username]">): Promise<Metadata> {
  const { username } = await params;
  return { title: username };
}

export default function ProfilePage({ params }: PageProps<"/u/[username]">) {
  return (
    <Suspense fallback={<div className="h-40 animate-pulse rounded-xl bg-surface" />}>
      <ProfileCard params={params} />
    </Suspense>
  );
}

async function ProfileCard({ params }: { params: PageProps<"/u/[username]">["params"] }) {
  const { username } = await params;
  const supabase = await createClient();
  const { data } = await supabase.from("profiles").select("*").eq("username", username).maybeSingle();
  if (!data) notFound();
  const p = data as Profile;

  return (
    <article className="grid gap-4 rounded-xl border border-line bg-surface p-6">
      <header>
        <h1 className="text-3xl font-extrabold tracking-tight">{p.display_name || p.username}</h1>
        <p className="text-dim">
          @{p.username}
          {p.country ? ` · ${p.country}` : ""} · joined {new Date(p.created_at).toLocaleDateString("en", { month: "long", year: "numeric" })}
        </p>
      </header>
      {p.bio && <p className="whitespace-pre-line">{p.bio}</p>}
    </article>
  );
}
