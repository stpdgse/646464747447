import type { Metadata } from "next";
import { PIECE_SETS } from "@/lib/chess/themes";
import { SITE } from "@/lib/site";

export const metadata: Metadata = { title: "Credits" };

const LIBS = [
  { name: "Chessground", by: "Lichess", license: "GPL-3.0+", url: "https://github.com/lichess-org/chessground", what: "the board" },
  { name: "chessops", by: "Niklas Fiekas", license: "GPL-3.0+", url: "https://github.com/niklasf/chessops", what: "chess rules, Chess960, PGN and FEN" },
  { name: "Stockfish", by: "The Stockfish developers", license: "GPL-3.0", url: "https://stockfishchess.org", what: "the computer opponent and analysis" },
  { name: "Lichess chess-openings", by: "Lichess contributors", license: "CC0", url: "https://github.com/lichess-org/chess-openings", what: "opening names and the opening book" },
  { name: "Lichess puzzle database", by: "Lichess contributors", license: "CC0", url: "https://database.lichess.org/#puzzles", what: "the puzzles" },
  { name: "Glicko-2", by: "Mark Glickman", license: "public algorithm", url: "http://www.glicko.net/glicko.html", what: "player ratings" },
];

export default function CreditsPage() {
  return (
    <div className="mx-auto grid max-w-3xl gap-8">
      <header className="grid gap-2">
        <h1 className="text-3xl font-extrabold tracking-tight">Credits</h1>
        <p className="text-dim">{SITE.name} stands on the work of many generous people.</p>
      </header>
      <section aria-labelledby="libs" className="grid gap-3">
        <h2 id="libs" className="text-xl font-bold">Software</h2>
        <ul className="grid gap-2">
          {LIBS.map((l) => (
            <li key={l.name} className="rounded-lg border border-line bg-surface p-3 text-sm">
              <a href={l.url} className="font-semibold text-brand underline">{l.name}</a> by {l.by} ({l.license}): {l.what}.
            </li>
          ))}
        </ul>
      </section>
      <section aria-labelledby="pieces" className="grid gap-3">
        <h2 id="pieces" className="text-xl font-bold">Piece sets</h2>
        <ul className="grid gap-2">
          {PIECE_SETS.map((p) => (
            <li key={p.id} className="flex items-center gap-3 rounded-lg border border-line bg-surface p-3 text-sm">
              {/* eslint-disable-next-line @next/next/no-img-element -- tiny local SVGs */}
              <img src={`/pieces/${p.id}/wN.svg`} alt="" className="size-10 shrink-0 rounded bg-surface-2" />
              <span>
                <strong>{p.label}</strong> by <a href={p.source} className="text-brand underline">{p.author}</a> ({p.license})
              </span>
            </li>
          ))}
        </ul>
        <p className="text-xs text-dim">Piece graphics come from the Lichess collection and keep their original licences.</p>
      </section>
    </div>
  );
}
