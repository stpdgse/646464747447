import type { Metadata } from "next";
import { ComingSoon } from "@/components/ComingSoon";
export const metadata: Metadata = { title: "Watch" };
export default function Page() {
  return <ComingSoon title="Watch" blurb="Spectate live games between your classmates." milestone="M4 and M10" />;
}
