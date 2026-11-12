import Link from "next/link";

export function ComingSoon({ title, blurb, milestone }: { title: string; blurb: string; milestone: string }) {
  return (
    <div className="mx-auto grid max-w-xl gap-4 pt-10 text-center">
      <h1 className="text-3xl font-extrabold tracking-tight">{title}</h1>
      <p className="text-dim">{blurb}</p>
      <p className="text-sm text-dim">Coming in {milestone}.</p>
      <Link href="/play/board" className="mx-auto rounded-md bg-brand px-5 py-2.5 font-semibold text-brand-ink hover:bg-brand-hi">
        Try the analysis board meanwhile
      </Link>
    </div>
  );
}
