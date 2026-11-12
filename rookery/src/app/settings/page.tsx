import type { Metadata } from "next";
import { Suspense } from "react";
import { requireProfile } from "@/lib/auth";
import { updateProfile } from "@/app/actions/profile";
import { ProfileForm } from "@/components/ProfileForm";

export const metadata: Metadata = { title: "Settings" };

export default function SettingsPage() {
  return (
    <div className="mx-auto grid max-w-lg gap-6">
      <h1 className="text-3xl font-extrabold tracking-tight">Settings</h1>
      <Suspense fallback={<div className="h-64 animate-pulse rounded-xl bg-surface" />}>
        <SettingsForm />
      </Suspense>
    </div>
  );
}

async function SettingsForm() {
  const profile = await requireProfile();
  return (
    <ProfileForm
      action={updateProfile}
      initial={{ display_name: profile.display_name ?? "", bio: profile.bio, country: profile.country ?? "" }}
    />
  );
}
