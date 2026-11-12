import type { Metadata } from "next";
import { ComingSoon } from "@/components/ComingSoon";
export const metadata: Metadata = { title: "Puzzles" };
export default function Page() {
  return <ComingSoon title="Puzzles" blurb="Rated tactics, Puzzle Rush and a daily puzzle." milestone="M7" />;
}
