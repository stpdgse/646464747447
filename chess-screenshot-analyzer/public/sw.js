// Offline cache: the app, engine and piece images are cached on first use, so after one
// online visit it works with no connection (e.g. on a plane).
const CACHE = 'chess-coach-v1';
const PRECACHE = ['./', './engine/stockfish-19-lite-single.js', './engine/stockfish-19-lite-single.wasm', './manifest.webmanifest', './icons/icon.svg'];

self.addEventListener('install', (e) => {
  e.waitUntil(caches.open(CACHE).then((c) => c.addAll(PRECACHE)).then(() => self.skipWaiting()));
});
self.addEventListener('activate', (e) => {
  e.waitUntil(caches.keys().then((ks) => Promise.all(ks.filter((k) => k !== CACHE).map((k) => caches.delete(k)))).then(() => self.clients.claim()));
});
self.addEventListener('fetch', (e) => {
  const req = e.request;
  if (req.method !== 'GET' || new URL(req.url).origin !== location.origin) return;
  const isPage = req.mode === 'navigate';
  e.respondWith((async () => {
    const cache = await caches.open(CACHE);
    const hit = await cache.match(req, { ignoreSearch: false });
    // Pages: network first (so updates arrive), fall back to cache. Everything else: cache first.
    if (isPage) {
      try { const r = await fetch(req); cache.put(req, r.clone()); return r; } catch { return hit || (await cache.match('./')); }
    }
    if (hit) return hit;
    const r = await fetch(req);
    if (r.ok) cache.put(req, r.clone());
    return r;
  })());
});
