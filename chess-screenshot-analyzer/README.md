# Screenshot Chess Coach

Upload a Chess.com screenshot, say whether you are White or Black, and get Stockfish's best moves.
It is a static web app (installable to the iPad Home Screen) and everything runs on the device —
the screenshot never leaves the browser, and after the first load it works offline.

## How it works

1. **Board detection** (`src/detect.js`) – finds the 8×8 grid from the periodic colour edges of the
   checkerboard, resolves which grid lines were found by checking the region really is a two-colour
   board, then refines to sub-pixel precision. Dragging the box re-snaps it onto the grid.
2. **Piece recognition** (`src/recognize.js`) – per square, estimates the background colour from the
   corners (handles light/dark squares and move highlights) and template-matches 12 piece images ×
   26 Chess.com piece sets. The set that explains the whole board wins; uncertain squares are
   outlined in amber and any square can be fixed by tapping it.
3. **Position** (`src/board.js`) – orientation from the colour you pick, castling rights inferred from
   king/rook placement, legality checked with chess.js.
4. **Engine** (`src/engine.js`) – Stockfish 19 (WASM, lite network) in a Web Worker, MultiPV for the
   top lines, with an asm.js fallback for browsers without WebAssembly.

## Develop

```bash
npm install
npm run dev        # http://localhost:5173
npm run build      # static site in dist/
npm test           # synthetic screenshots: detection + recognition accuracy
npm run test:real  # the real iPad screenshot in test/fixtures
```

End-to-end browser tests (need Playwright + Chromium):

```bash
npm run build && npx vite preview --port 4173 &
NODE_PATH=$(npm root -g) node test/e2e.cjs test/fixtures/ipad-start.png w /tmp/out
NODE_PATH=$(npm root -g) node test/e2e-interact.cjs
```

## Notes

- Serve over HTTPS (or localhost) for the service worker / offline mode and Home Screen install.
- Piece sets live as 12 PNGs each in `assets/pieces-src/`; `node scripts/make-sprites.js` packs them into one sprite per set in `public/pieces/` (already committed).
- `public/pieces/` holds Chess.com's piece artwork, used only to recognise pieces in your own
  screenshots. Keep deployments private/personal; do not redistribute them publicly.
- En passant is not detectable from a single image; the engine assumes none.
- Use it for studying your own and finished games. Using an engine during live rated games violates
  Chess.com's fair-play rules.
- Stockfish and Stockfish.js are GPL-3.0; this project is therefore GPL-3.0-or-later.
