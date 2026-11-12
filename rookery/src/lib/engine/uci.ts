/**
 * A small, transport-agnostic UCI client. In the browser the transport is a Web Worker running Stockfish;
 * in tests it is a Node child process. Everything here is plain TypeScript with no DOM dependency.
 */

export interface Transport {
  send(command: string): void;
  onLine(callback: (line: string) => void): void;
  terminate(): void;
}

/** One analysed move. `cp` is from the side-to-move's point of view; mates are folded into large cp values. */
export type Line = { uci: string; cp: number; mate: number | null; depth: number; multipv: number; pv: string[] };

export type SearchOptions = {
  fen: string;
  moves?: string[];
  depth?: number;
  movetime?: number;
  multipv?: number;
  /** 1320-3190: engine plays at about this strength. Omit for full strength (or use `skill`). */
  elo?: number;
  /** 0-20 (Stockfish "Skill Level"), used when `elo` is not set. */
  skill?: number;
};

export type SearchResult = { best: string | null; lines: Line[] };

const MATE_CP = 100_000;
const INFO_RE = /^info .*\bdepth (\d+).*\bmultipv (\d+) score (cp|mate) (-?\d+)(?: (?:lowerbound|upperbound))?.* pv (.+)$/;

/** Parse one `info ...` line, or return null if it carries no score/pv. */
export function parseInfo(line: string): Line | null {
  const m = INFO_RE.exec(line);
  if (!m) return null;
  const pv = m[5].trim().split(/\s+/);
  const value = Number(m[4]);
  const isMate = m[3] === "mate";
  return {
    uci: pv[0],
    cp: isMate ? Math.sign(value) * (MATE_CP - Math.abs(value)) : value,
    mate: isMate ? value : null,
    depth: Number(m[1]),
    multipv: Number(m[2]),
    pv,
  };
}

export class UciEngine {
  private listeners = new Set<(line: string) => void>();
  private queue: Promise<unknown> = Promise.resolve();
  private current = new Map<string, string>();
  private ready: Promise<void>;
  private dead = false;

  constructor(private transport: Transport) {
    transport.onLine((line) => {
      for (const l of [...this.listeners]) l(line);
    });
    this.ready = this.handshake();
  }

  private waitFor(test: (line: string) => boolean, label: string, timeoutMs = 30_000): Promise<string> {
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        this.listeners.delete(handler);
        reject(new Error(`Engine did not answer (${label}).`));
      }, timeoutMs);
      const handler = (line: string) => {
        if (test(line)) {
          clearTimeout(timer);
          this.listeners.delete(handler);
          resolve(line);
        }
      };
      this.listeners.add(handler);
    });
  }

  private async handshake() {
    const ok = this.waitFor((l) => l === "uciok", "uci");
    this.transport.send("uci");
    await ok;
    // Chess960 mode makes castling "king takes own rook" for every game, which is how our records store it.
    this.setOption("UCI_Chess960", true);
    this.setOption("Hash", 16);
    await this.sync();
  }

  private setOption(name: string, value: string | number | boolean) {
    if (this.current.get(name) === String(value)) return;
    this.current.set(name, String(value));
    this.transport.send(`setoption name ${name} value ${value}`);
  }

  private async sync() {
    const ok = this.waitFor((l) => l === "readyok", "isready");
    this.transport.send("isready");
    await ok;
  }

  /** Resolves once the engine has loaded and answered the UCI handshake. */
  whenReady(): Promise<void> {
    return this.ready;
  }

  /** Searches one position. Calls are queued, so a second search waits for the first. */
  search(opts: SearchOptions, signal?: AbortSignal): Promise<SearchResult> {
    const run = async (): Promise<SearchResult> => {
      if (this.dead) throw new Error("Engine was shut down.");
      await this.ready;
      if (signal?.aborted) return { best: null, lines: [] };

      const multipv = Math.max(1, opts.multipv ?? 1);
      this.setOption("MultiPV", multipv);
      if (opts.elo !== undefined) {
        this.setOption("UCI_LimitStrength", true);
        this.setOption("UCI_Elo", Math.min(3190, Math.max(1320, Math.round(opts.elo))));
        this.setOption("Skill Level", 20);
      } else {
        this.setOption("UCI_LimitStrength", false);
        this.setOption("Skill Level", Math.min(20, Math.max(0, Math.round(opts.skill ?? 20))));
      }
      await this.sync();

      const best = new Map<number, Line>();
      const onInfo = (line: string) => {
        const info = parseInfo(line);
        if (!info) return;
        const prev = best.get(info.multipv);
        if (!prev || info.depth >= prev.depth) best.set(info.multipv, info);
      };
      this.listeners.add(onInfo);

      const moves = opts.moves?.length ? ` moves ${opts.moves.join(" ")}` : "";
      this.transport.send(`position fen ${opts.fen}${moves}`);
      const limit = [opts.depth ? `depth ${opts.depth}` : "", opts.movetime ? `movetime ${opts.movetime}` : ""].filter(Boolean).join(" ");

      const done = this.waitFor((l) => l.startsWith("bestmove"), "go", 120_000);
      const onAbort = () => this.transport.send("stop");
      signal?.addEventListener("abort", onAbort);
      this.transport.send(`go ${limit || "depth 12"}`);
      let bestLine: string;
      try {
        bestLine = await done;
      } finally {
        this.listeners.delete(onInfo);
        signal?.removeEventListener("abort", onAbort);
      }

      const token = bestLine.split(/\s+/)[1];
      const lines = [...best.values()].sort((a, b) => a.multipv - b.multipv);
      return { best: !token || token === "(none)" ? null : token, lines };
    };
    const result = this.queue.then(run, run);
    this.queue = result.catch(() => undefined);
    return result;
  }

  /** Forget the previous game's hash table. */
  async newGame() {
    await this.ready;
    this.transport.send("ucinewgame");
    await this.sync();
  }

  stop() {
    this.transport.send("stop");
  }

  terminate() {
    this.dead = true;
    this.transport.terminate();
  }
}

/** Browser transport: Stockfish running in a Web Worker served from /engine. */
export function createWorkerTransport(): Transport {
  const wasm = typeof WebAssembly === "object" && typeof WebAssembly.instantiate === "function";
  const worker = new Worker(wasm ? "/engine/stockfish-19-lite-single.js" : "/engine/stockfish-19-asm.js");
  let cb: (line: string) => void = () => {};
  worker.onmessage = (e: MessageEvent) => {
    if (typeof e.data === "string") cb(e.data);
  };
  return {
    send: (c) => worker.postMessage(c),
    onLine: (f) => {
      cb = f;
    },
    terminate: () => worker.terminate(),
  };
}
