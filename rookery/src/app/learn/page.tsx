import type { Metadata } from "next";
import { ComingSoon } from "@/components/ComingSoon";
export const metadata: Metadata = { title: "Learn" };
export default function Page() {
  return <ComingSoon title="Learn" blurb="Game review, an opening explorer and endgame practice." milestone="M8 and M9" />;
}
