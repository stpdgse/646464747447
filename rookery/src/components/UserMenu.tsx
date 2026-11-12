import Link from "next/link";
import { getCurrentProfile } from "@/lib/auth";
import { logout } from "@/app/actions/auth";

/** Reads the session, so it must render inside <Suspense> (see SiteHeader). */
export async function UserMenu() {
  const profile = await getCurrentProfile();

  if (!profile) {
    return (
      <>
        <Link href="/login" className="rounded-md px-3 py-1.5 text-sm font-medium text-dim hover:text-ink">
          Log in
        </Link>
        <Link
          href="/signup"
          className="rounded-md bg-brand px-3.5 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi"
        >
          Sign up
        </Link>
      </>
    );
  }

  return (
    <>
      <Link href={`/u/${profile.username}`} className="rounded-md px-3 py-1.5 text-sm font-semibold hover:bg-surface-2">
        {profile.username}
      </Link>
      <Link href="/settings" className="rounded-md px-3 py-1.5 text-sm text-dim hover:text-ink">
        Settings
      </Link>
      <form action={logout}>
        <button className="rounded-md px-3 py-1.5 text-sm text-dim hover:text-ink">Log out</button>
      </form>
    </>
  );
}

export function UserMenuFallback() {
  return <span aria-hidden className="h-8 w-28 animate-pulse rounded-md bg-surface-2" />;
}
