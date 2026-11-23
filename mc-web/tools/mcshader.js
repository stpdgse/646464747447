// Loads mcshader.wasm (glslang + SPIRV-Cross for wasm32-wasi) and exposes it as globalThis.mcshader.
// The module is built by work/shaderc/build_module.sh; it needs only a tiny WASI surface.
(function () {
  "use strict";
  async function load(bytesOrResponse) {
    let memory = null;
    const dv = () => new DataView(memory.buffer);
    const wasi = {
      fd_close: () => 0,
      fd_fdstat_get: (fd, p) => { dv().setUint8(p, fd <= 2 ? 2 : 4); return 0; },
      fd_seek: () => 70,
      fd_write: (fd, iovs, n, nw) => {
        let total = 0, text = "";
        const view = dv();
        for (let i = 0; i < n; i++) {
          const ptr = view.getUint32(iovs + i * 8, true), len = view.getUint32(iovs + i * 8 + 4, true);
          text += new TextDecoder().decode(new Uint8Array(memory.buffer, ptr, len));
          total += len;
        }
        if (text.trim()) console.log("[mcshader] " + text.replace(/\n$/, ""));
        view.setUint32(nw, total, true);
        return 0;
      },
      environ_sizes_get: (c, s) => { dv().setUint32(c, 0, true); dv().setUint32(s, 0, true); return 0; },
      environ_get: () => 0,
      proc_exit: (code) => { throw new Error("mcshader exited with code " + code); },
      clock_time_get: (id, prec, out) => { dv().setBigUint64(out, BigInt(Math.round(performance.now() * 1e6)), true); return 0; }
    };
    const source = bytesOrResponse;
    let result;
    if (typeof Response !== "undefined" && source instanceof Response) {
      result = await WebAssembly.instantiateStreaming(source, { wasi_snapshot_preview1: wasi });
    } else {
      result = await WebAssembly.instantiate(source, { wasi_snapshot_preview1: wasi });
    }
    const ex = result.instance.exports;
    memory = ex.memory;
    ex._initialize();
    ex.mc_init();
    const enc = new TextEncoder(), dec = new TextDecoder();
    const api = {
      exports: ex,
      get memory() { return memory; },
      u8() { return new Uint8Array(memory.buffer); },
      dv() { return new DataView(memory.buffer); },
      /** Copies a JS string into wasm memory as a NUL-terminated UTF-8 string; free with api.free. */
      cstr(s) {
        const b = enc.encode(s);
        const p = ex.malloc(b.length + 1);
        const u8 = new Uint8Array(memory.buffer);
        u8.set(b, p);
        u8[p + b.length] = 0;
        return p;
      },
      str(p) {
        if (!p) return null;
        const u8 = new Uint8Array(memory.buffer);
        let e = p;
        while (u8[e] !== 0) e++;
        return dec.decode(u8.subarray(p, e));
      },
      malloc: (n) => ex.malloc(n),
      free: (p) => ex.free(p),
      /** GLSL (Vulkan flavour) to SPIR-V. Returns {words: Uint32Array} or {error: string}. */
      compile(source, stage, preamble, autoBind) {
        const s = api.cstr(source), pr = api.cstr(preamble || "");
        const n = ex.mc_compile(s, stage, pr, autoBind ? 1 : 0);
        ex.free(s); ex.free(pr);
        if (n < 0) return { error: api.str(ex.mc_log()) };
        const p = ex.mc_spv();
        return { words: new Uint32Array(memory.buffer.slice(p, p + n * 4)) };
      }
    };
    return api;
  }
  globalThis.mcshaderLoad = load;
})();
