import Link from "next/link";
import { NAV, SITE } from "@/lib/site";

export function SiteHeader() {
  return (
    <header className="sticky top-0 z-40 border-b border-line bg-surface/95 backdrop-blur">
      <div className="mx-auto flex h-14 max-w-6xl items-center gap-6 px-4">
        <Link href="/" className="flex items-center gap-2 text-lg font-bold tracking-tight">
          <span aria-hidden className="text-2xl leading-none text-brand">♜</span>
          {SITE.name}
        </Link>
        <nav aria-label="Main" className="hidden gap-1 sm:flex">
          {NAV.map((item) => (
            <Link
              key={item.href}
              href={item.href}
              className="rounded-md px-3 py-1.5 text-sm font-medium text-dim transition hover:bg-surface-2 hover:text-ink"
            >
              {item.label}
            </Link>
          ))}
        </nav>
        <div className="ml-auto flex items-center gap-2">
          <Link
            href="/login"
            className="rounded-md px-3 py-1.5 text-sm font-medium text-dim hover:text-ink"
          >
            Log in
          </Link>
          <Link
            href="/signup"
            className="rounded-md bg-brand px-3.5 py-1.5 text-sm font-semibold text-brand-ink hover:bg-brand-hi"
          >
            Sign up
          </Link>
        </div>
      </div>
    </header>
  );
}
