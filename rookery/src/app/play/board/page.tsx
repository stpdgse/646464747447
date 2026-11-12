import type { Metadata } from "next";
import { Workspace } from "@/components/chess/Workspace";

export const metadata: Metadata = {
  title: "Analysis board",
  description: "Set up positions, play both sides, step through games and try Chess960, King of the Hill and Three-check.",
};

export default function BoardPage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Analysis board</h1>
      <Workspace />
    </div>
  );
}
