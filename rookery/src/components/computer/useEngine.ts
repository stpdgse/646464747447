"use client";

import { useEffect, useState } from "react";
import { UciEngine, createWorkerTransport } from "@/lib/engine/uci";

export type EngineState =
  | { status: "loading" }
  | { status: "ready"; engine: UciEngine }
  | { status: "error"; message: string };

/** Starts Stockfish in a Web Worker for the lifetime of the component. */
export function useEngine(): EngineState {
  const [state, setState] = useState<EngineState>({ status: "loading" });

  useEffect(() => {
    let dead = false;
    let engine: UciEngine | null = null;
    const fail = (message: string) => queueMicrotask(() => !dead && setState({ status: "error", message }));
    try {
      engine = new UciEngine(createWorkerTransport());
    } catch {
      fail("Your browser would not start the chess engine.");
      return () => {
        dead = true;
      };
    }
    engine
      .whenReady()
      .then(() => !dead && engine && setState({ status: "ready", engine }))
      .catch(() => fail("The chess engine did not start. A school filter may be blocking it."));
    return () => {
      dead = true;
      engine?.terminate();
    };
  }, []);

  return state;
}
