// Thin async wrapper around Stockfish.js (WASM, single-threaded "lite" build) running in a
// Web Worker. Falls back to the slower asm.js build if WebAssembly is unavailable.

const FILES = ['stockfish-19-lite-single.js', 'stockfish-19-asm.js'];

export class Engine {
  constructor(base = './engine/') {
    this.base = base;
    this.worker = null;
    this.job = null;
    this.name = '';
  }

  async start() {
    let lastErr;
    for (const file of FILES) {
      try {
        await this._boot(file);
        this.name = file.includes('asm') ? 'Stockfish 19 (asm.js fallback)' : 'Stockfish 19 (WASM)';
        return this.name;
      } catch (e) {
        lastErr = e;
        this.terminate();
      }
    }
    throw lastErr || new Error('Could not start Stockfish');
  }

  _boot(file) {
    return new Promise((resolve, reject) => {
      let w;
      try { w = new Worker(this.base + file); } catch (e) { reject(e); return; }
      this.worker = w;
      const timer = setTimeout(() => reject(new Error('Engine start timed out')), 30000);
      w.onerror = (e) => { clearTimeout(timer); reject(new Error(e.message || 'Engine failed to load')); };
      w.onmessage = (e) => {
        const line = String(e.data);
        if (line.startsWith('uciok')) {
          clearTimeout(timer);
          w.onmessage = (ev) => this._line(String(ev.data));
          w.onerror = null;
          this._send('setoption name Hash value 32');
          resolve();
        }
      };
      w.postMessage('uci');
    });
  }

  _send(cmd) { this.worker.postMessage(cmd); }

  terminate() {
    if (this.worker) { this.worker.terminate(); this.worker = null; }
    if (this.job) { this.job.finish(true); this.job = null; }
  }

  _line(line) {
    const job = this.job;
    if (!job) return;
    if (line.startsWith('bestmove')) { job.finish(false); return; }
    if (!line.startsWith('info') || !line.includes(' pv ') || !line.includes(' score ')) return;
    const t = line.split(' ');
    const get = (key) => { const i = t.indexOf(key); return i < 0 ? undefined : t[i + 1]; };
    const kind = t[t.indexOf('score') + 1];
    const raw = parseInt(t[t.indexOf('score') + 2], 10);
    const entry = {
      multipv: parseInt(get('multipv') || '1', 10),
      depth: parseInt(get('depth'), 10),
      cp: kind === 'cp' ? raw : null,
      mate: kind === 'mate' ? raw : null,
      bound: line.includes(' lowerbound') ? 'lower' : line.includes(' upperbound') ? 'upper' : null,
      pv: t.slice(t.indexOf('pv') + 1),
    };
    if (entry.bound) return; // ignore aspiration-window fail-high/low lines
    job.lines.set(entry.multipv, entry);
    job.depth = Math.max(job.depth, entry.depth);
    job.onUpdate?.(job.sorted(), job.depth, false);
  }

  /** Stop the running search (if any) and wait until the engine is idle. */
  async stop() {
    const job = this.job;
    if (!job) return;
    this._send('stop');
    await job.done;
  }

  /**
   * @param {{fen:string, depth?:number, multipv?:number, movetime?:number}} o
   * @param {(lines:Array, depth:number, final:boolean)=>void} onUpdate
   * @returns {Promise<{lines:Array, depth:number, stopped:boolean}>}
   */
  async analyze(o, onUpdate) {
    if (!this.worker) throw new Error('Engine not running');
    await this.stop();
    let resolveDone;
    const done = new Promise((r) => { resolveDone = r; });
    const job = {
      lines: new Map(),
      depth: 0,
      onUpdate,
      done,
      sorted() { return [...this.lines.values()].sort((a, b) => a.multipv - b.multipv); },
      finish: (stopped) => {
        if (this.job === job) this.job = null;
        onUpdate?.(job.sorted(), job.depth, true);
        resolveDone({ lines: job.sorted(), depth: job.depth, stopped });
      },
    };
    this.job = job;
    this._send(`setoption name MultiPV value ${o.multipv || 3}`);
    this._send(`position fen ${o.fen}`);
    let go = `go depth ${o.depth || 18}`;
    if (o.movetime) go += ` movetime ${o.movetime}`;
    this._send(go);
    return done;
  }
}
