"""Extract TeaVM's WASM GC runtime script and apply the mc-web patches.

usage: make_runtime.py TEAVM_CORE_JAR OUT_JS

Patches (each is a no-op if the text is not found, and reports that):
  * createWeakRef: Java allows `new WeakReference<>(null)`, JavaScript's WeakRef does not.
"""
import sys
import zipfile

jar, out = sys.argv[1], sys.argv[2]
src = zipfile.ZipFile(jar).read('org/teavm/backend/wasm/wasm-gc-runtime.js').decode('utf8')

old = '''      createWeakRef(value, ref, queue) {
        if (queue !== null) {'''
new = '''      createWeakRef(value, ref, queue) {
        if (value === null || value === undefined) {
          return emptyWeakRef;
        }
        if (queue !== null) {'''
if old in src:
    src = src.replace(old, new, 1)
    src = src.replace('    imports.teavm = {', '    const emptyWeakRef = { deref() { return undefined; } };\n    imports.teavm = {', 1)
    print('patched: null-safe createWeakRef')
else:
    print('WARNING: createWeakRef not found; runtime layout changed?')
old2 = "      offer(instance, fn, time) {\n"
new2 = ("      offer(instance, fn, time) {\n"
        "        if (globalThis.mcwebDebugAsync && (globalThis.mcwebAsyncLogged = (globalThis.mcwebAsyncLogged || 0) + 1) <= 3) {\n"
        "          console.warn('[async offer #' + globalThis.mcwebAsyncLogged + ']\\n' + new Error().stack);\n"
        "        }\n")
if old2 in src:
    src = src.replace(old2, new2, 1)
    print('patched: optional async-offer logging (set globalThis.mcwebDebugAsync)')
else:
    print('WARNING: asyncImports.offer not found')
open(out, 'w').write(src)
