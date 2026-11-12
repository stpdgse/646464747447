// Copies the Stockfish engine files from node_modules into public/engine so the browser can load them as
// same-origin static files. Runs on every `npm install` (and so on Vercel). Files are git-ignored.
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";

const src = join(dirname(new URL(import.meta.url).pathname), "..", "node_modules", "stockfish", "bin");
const dest = join(dirname(new URL(import.meta.url).pathname), "..", "public", "engine");
const files = ["stockfish-19-lite-single.js", "stockfish-19-lite-single.wasm", "stockfish-19-asm.js"];

if (!existsSync(src)) {
  console.warn("copy-engine: stockfish is not installed yet, skipping.");
  process.exit(0);
}
mkdirSync(dest, { recursive: true });
for (const f of files) copyFileSync(join(src, f), join(dest, f));
console.log(`copy-engine: copied ${files.length} engine files to public/engine`);
