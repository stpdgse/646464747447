import type { Metadata } from "next";
import Link from "next/link";
import { Suspense } from "react";
import { FriendsPanel } from "@/components/social/FriendsPanel";
import { getCurrentProfile } from "@/lib/auth";

export const metadata: Metadata = { title: "Friends" };

export default function FriendsPage() {
  return (
    <div className="grid gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Friends</h1>
      <Suspense fallback={<div className="h-48 animate-pulse rounded-xl bg-surface" />}>
        <Loader />
      </Suspense>
    </div>
  );
}

async function Loader() {
  const me = await getCurrentProfile();
  if (!me) {
    return <p className="text-dim"><Link className="text-brand underline" href="/login">Log in</Link> to add friends and see who is online.</p>;
  }
  return <FriendsPanel viewerId={me.id} />;
}
