import { ACHIEVEMENTS, CATEGORIES } from "@/lib/achievements";

/** Every badge, earned ones in colour with the date, locked ones dashed with what to do. */
export function AchievementsGrid({ earned }: { earned: Record<string, string> }) {
  const count = ACHIEVEMENTS.filter((a) => earned[a.key]).length;
  return (
    <div className="grid gap-5">
      <p className="text-sm text-dim"><strong className="text-ink">{count}</strong> of {ACHIEVEMENTS.length} badges earned</p>
      {CATEGORIES.map((cat) => (
        <div key={cat} className="grid gap-2">
          <h3 className="text-sm font-bold uppercase tracking-wide text-dim">{cat}</h3>
          <ul className="grid gap-2 sm:grid-cols-2 lg:grid-cols-3">
            {ACHIEVEMENTS.filter((a) => a.category === cat).map((a) => {
              const when = earned[a.key];
              return (
                <li key={a.key} className={`flex items-start gap-3 rounded-xl border p-3 ${when ? "border-brand bg-brand/10" : "border-dashed border-line bg-surface"}`}>
                  <span aria-hidden className={`text-2xl ${when ? "" : "grayscale"}`}>{a.icon}</span>
                  <span className="min-w-0">
                    <span className="block text-sm font-bold">{a.name}{when ? <span className="sr-only"> (earned)</span> : <span className="sr-only"> (not yet earned)</span>}</span>
                    <span className="block text-xs text-dim">{a.desc}</span>
                    {when && <span className="block text-xs font-semibold text-brand">Earned {when.slice(0, 10)}</span>}
                  </span>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </div>
  );
}
