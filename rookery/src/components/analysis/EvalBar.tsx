import { formatEval, winPercent } from "@/lib/analysis/analysis";

/** A vertical bar showing who is winning. White's share fills from White's side of the board. */
export function EvalBar({ cp, mate, orientation }: { cp: number | null; mate: number | null; orientation: "white" | "black" }) {
  const known = cp !== null;
  const white = known ? winPercent(cp) : 50;
  const label = known ? formatEval({ cp, mate }) : "";
  const whiteOnTop = orientation === "black";
  const whiteLeads = known && white >= 50;
  return (
    <div
      role="img"
      aria-label={known ? `Evaluation ${label}. White has a ${Math.round(white)} percent chance.` : "Engine off"}
      className="relative w-6 shrink-0 overflow-hidden rounded-md border border-line bg-[#2b2926]"
    >
      <div className="absolute inset-x-0 bg-[#f1efe9] transition-[height] duration-300" style={whiteOnTop ? { top: 0, height: `${white}%` } : { bottom: 0, height: `${white}%` }} />
      <div className="absolute inset-x-0 top-1/2 h-px bg-brand/60" aria-hidden />
      {known && (
        <span
          className={`absolute inset-x-0 text-center text-[10px] font-bold leading-none ${whiteLeads ? "text-[#2b2926]" : "text-[#f1efe9]"}`}
          style={(whiteLeads !== whiteOnTop) ? { bottom: 4 } : { top: 4 }}
        >
          {label}
        </span>
      )}
    </div>
  );
}
