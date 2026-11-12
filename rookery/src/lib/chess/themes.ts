export type BoardTheme = { id: string; label: string; light: string; dark: string };

export const BOARD_THEMES: BoardTheme[] = [
  { id: "green", label: "Green", light: "#eeeed2", dark: "#769656" },
  { id: "brown", label: "Brown", light: "#f0d9b5", dark: "#b58863" },
  { id: "blue", label: "Blue", light: "#dee3e6", dark: "#8ca2ad" },
  { id: "purple", label: "Purple", light: "#e8e0f0", dark: "#8877b3" },
  { id: "slate", label: "Slate", light: "#b8bcc4", dark: "#5d6575" },
  { id: "coral", label: "Coral", light: "#f5e6d8", dark: "#d17b5b" },
];

export type PieceSet = { id: string; label: string; author: string; license: string; source: string };

/** Every set is licensed for free reuse (see /credits). Files live in public/pieces/<id>/. */
export const PIECE_SETS: PieceSet[] = [
  { id: "cburnett", label: "Classic", author: "Colin M.L. Burnett", license: "GPL-2.0+", source: "https://en.wikipedia.org/wiki/User:Cburnett" },
  { id: "merida", label: "Merida", author: "Armando Hernandez Marroquin", license: "GPL-2.0+", source: "https://github.com/lichess-org/lila/tree/master/public/piece/merida" },
  { id: "chessnut", label: "Chessnut", author: "Alexis Luengas", license: "Apache-2.0", source: "https://github.com/LexLuengas/chessnut-pieces" },
  { id: "mpchess", label: "MP Chess", author: "Maxime Chupin", license: "GPL-3.0+", source: "https://github.com/chupinmaxime" },
  { id: "rhosgfx", label: "Rhos", author: "RhosGFX", license: "CC0 1.0", source: "https://rhosgfx.itch.io/" },
  { id: "fantasy", label: "Fantasy", author: "Maurizio Monge", license: "MIT", source: "https://github.com/maurimo/chess-art" },
  { id: "celtic", label: "Celtic", author: "Maurizio Monge", license: "MIT", source: "https://github.com/maurimo/chess-art" },
  { id: "papercut", label: "Papercut", author: "Nikolay Anzarov", license: "CC BY 4.0", source: "https://nikoichu.itch.io/" },
];

export type BoardSettings = {
  boardTheme: string;
  pieceSet: string;
  coordinates: boolean;
  animation: boolean;
  highlights: boolean;
};

export const DEFAULT_SETTINGS: BoardSettings = {
  boardTheme: "green",
  pieceSet: "cburnett",
  coordinates: true,
  animation: true,
  highlights: true,
};

export function normalizeSettings(raw: unknown): BoardSettings {
  const s = (typeof raw === "object" && raw !== null ? raw : {}) as Record<string, unknown>;
  return {
    boardTheme: BOARD_THEMES.some((t) => t.id === s.boardTheme) ? (s.boardTheme as string) : DEFAULT_SETTINGS.boardTheme,
    pieceSet: PIECE_SETS.some((p) => p.id === s.pieceSet) ? (s.pieceSet as string) : DEFAULT_SETTINGS.pieceSet,
    coordinates: typeof s.coordinates === "boolean" ? s.coordinates : DEFAULT_SETTINGS.coordinates,
    animation: typeof s.animation === "boolean" ? s.animation : DEFAULT_SETTINGS.animation,
    highlights: typeof s.highlights === "boolean" ? s.highlights : DEFAULT_SETTINGS.highlights,
  };
}
