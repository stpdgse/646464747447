# Deploying Rookery to Vercel

Takes about 5 minutes. You do this once, then every push deploys automatically.

## 1. Connect GitHub to Vercel (once)
Vercel dashboard → your avatar → **Account Settings → Authentication / Git Integrations** → connect **GitHub**,
and allow access to the `stpdgse/646464747447` repository.

## 2. Import the project
1. Vercel dashboard → **Add New… → Project** → pick `stpdgse/646464747447`.
2. **Root Directory:** click *Edit* and choose `rookery`. (The repo also holds other projects.)
3. **Framework Preset:** Next.js (auto-detected). Leave build settings alone.
4. **Environment Variables:** add both, for all environments:

   | Name | Value |
   |---|---|
   | `NEXT_PUBLIC_SUPABASE_URL` | `https://ultpyrvfzdqamprglnxd.supabase.co` |
   | `NEXT_PUBLIC_SUPABASE_PUBLISHABLE_KEY` | `sb_publishable_qb3qtdDNm_6apNtC37vQ_Q_bfgzVwy7` |
   | `GAME_SERVER_SECRET` | a long random string (Sensitive). Register its SHA-256 in Supabase, see README. |

   Both are public by design (they ship to the browser). Row Level Security is what protects the data.
5. Click **Deploy**.

## 3. Let classmates in (important)
Vercel protects **preview** deployments (any branch other than `main`) behind a Vercel login by default.
Classmates would hit a login wall. Two ways out:
- **Merge the pull request into `main`.** `main` is the *production* branch, which is public. Recommended.
- Or: project **Settings → Deployment Protection** → set *Vercel Authentication* to **Disabled**.

## 4. Supabase redirect settings
Supabase dashboard → **Authentication → URL Configuration** → set **Site URL** to your Vercel URL
(for example `https://rookery.vercel.app`). Username accounts work without it, but later features that
build links need it.
