// usage: node [--stack-size=N] runnode.js DIR WASM_FILE [timeoutSeconds]
Error.stackTraceLimit = 100;
// Runs a TeaVM WASM GC build in Node (same V8 engine as Chrome) with a minimal browser shim.
const fs = require('fs'), path = require('path'), vm = require('vm');
const dir = process.argv[2], wasm = process.argv[3], secs = +(process.argv[4] || 60);
globalThis.window = globalThis;
globalThis.mcwebDebugAsync = !!process.env.MCWEB_DEBUG_ASYNC;
globalThis.document = { addEventListener() {}, createElement: () => ({ getContext: () => null, style: {} }), body: {}, getElementById: () => null };
globalThis.navigator = globalThis.navigator || { userAgent: 'node' };
const origFetch = globalThis.fetch;
globalThis.fetch = async (url) => {
  const f = path.join(dir, String(url).replace(/^.*\//, ''));
  return new Response(fs.readFileSync(f), { headers: { 'content-type': f.endsWith('.wasm') ? 'application/wasm' : 'text/plain' } });
};
vm.runInThisContext(fs.readFileSync(path.join(__dirname, 'mcweb.js'), 'utf8'));
const rt = fs.readFileSync(path.join(dir, 'game.wasm-runtime.js'), 'utf8');
vm.runInThisContext(rt, { importModuleDynamically: vm.constants.USE_MAIN_CONTEXT_DEFAULT_LOADER });
(async () => {
  try {
    const arch = path.join(dir, 'client-resources.zip');
    if (fs.existsSync(arch)) {
      const buf = fs.readFileSync(arch);
      globalThis.mcwebArchive = new Int8Array(buf.buffer, buf.byteOffset, buf.length);
    }
    const t = await TeaVM.wasmGC.load(path.join(dir, wasm));
    console.log('loaded; calling main');
    await t.exports.main([]);
    console.log('main returned');
  } catch (e) {
    console.log('ERR', e && e.stack ? e.stack.split('\n').slice(0, 40).join('\n') : e);
  }
})();
setTimeout(() => { console.log('timeout reached'); process.exit(0); }, secs * 1000);
