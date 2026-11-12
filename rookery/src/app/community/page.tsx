import type { Metadata } from "next";
import { ComingSoon } from "@/components/ComingSoon";
export const metadata: Metadata = { title: "Community" };
export default function Page() {
  return <ComingSoon title="Community" blurb="Friends, clubs, leaderboards and the forum." milestone="M5, M10 and M11" />;
}
