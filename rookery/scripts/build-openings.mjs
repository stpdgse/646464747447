// Builds src/data/openings.json from the Lichess chess-openings TSV files (CC0, https://github.com/lichess-org/chess-openings).
// Usage: node scripts/build-openings.mjs <folder with a.tsv ... e.tsv>
import fs from "node:fs";
import path from "node:path";
import { Chess } from "chessops/chess";
import { parseFen, INITIAL_FEN } from "chessops/fen";
import { parseSan } from "chessops/san";
import { makeUci } from "chessops/util";

const dir = process.argv[2];
if (!dir) throw new Error("pass the folder containing the .tsv files");
const out = [];
let skipped = 0;
for (const f of ["a", "b", "c", "d", "e"]) {
  const lines = fs.readFileSync(path.join(dir, `${f}.tsv`), "utf8").split("\n").slice(1).filter(Boolean);
  for (const line of lines) {
    const [eco, name, pgn] = line.split("\t");
    const pos = Chess.fromSetup(parseFen(INITIAL_FEN).unwrap()).unwrap();
    const ucis = [];
    let ok = true;
    for (const tok of pgn.replace(/\d+\.(\.\.)?/g, " ").split(/\s+/).filter(Boolean)) {
      const move = parseSan(pos, tok);
      if (!move) { ok = false; break; }
      ucis.push(makeUci(move));
      pos.play(move);
    }
    if (!ok || ucis.length === 0) { skipped++; continue; }
    out.push([eco, name, ucis.join(" ")]);
  }
}
fs.writeFileSync("src/data/openings.json", JSON.stringify(out));
console.log(`wrote ${out.length} openings (${skipped} skipped)`);
