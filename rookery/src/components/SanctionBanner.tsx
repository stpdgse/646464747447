import { getCurrentProfile } from "@/lib/auth";
import { fmtDate, isFuture } from "@/lib/social";

/** Tells a muted or banned player why things don't work. Reads the session, so render inside <Suspense>. */
export async function SanctionBanner() {
  const me = await getCurrentProfile();
  if (!me) return null;
  const banned = isFuture(me.banned_until);
  const muted = isFuture(me.muted_until);
  if (!banned && !muted) return null;
  return (
    <div role="alert" className="border-b border-danger/50 bg-danger/10 px-4 py-2 text-center text-sm text-danger">
      {banned
        ? `Your account is suspended until ${fmtDate(me.banned_until!)}. You can't play or post.`
        : `You are muted until ${fmtDate(me.muted_until!)}. You can't chat or post.`}
    </div>
  );
}
