import type { Metadata } from "next";
import { WatchList } from "@/components/live/WatchList";

export const metadata: Metadata = { title: "Watch", description: "Watch live games between classmates." };

export default function WatchPage() {
  return (
    <div className="grid gap-4">
      <h1 className="text-3xl font-extrabold tracking-tight">Watch</h1>
      <WatchList />
    </div>
  );
}
