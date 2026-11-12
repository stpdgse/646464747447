/** Single source of truth for the site's identity. Rename the site by editing this file. */
export const SITE = {
  name: "Rookery",
  tagline: "Chess for the whole class.",
  description:
    "Play live chess, solve puzzles and review your games with your classmates. No downloads, no fuss.",
} as const;

export const NAV = [
  { href: "/play", label: "Play" },
  { href: "/puzzles", label: "Puzzles" },
  { href: "/learn", label: "Learn" },
  { href: "/watch", label: "Watch" },
  { href: "/community", label: "Community" },
] as const;
