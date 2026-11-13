/* Rookery service worker: an offline page, cached static files, and the last copy of the play pages so a game against the
   computer can be reopened with no connection. Data (Supabase) is never cached; online games need the network. */
const VERSION = "v2";
const STATIC = `rookery-static-${VERSION}`;
const PAGES = `rookery-pages-${VERSION}`;
const PRECACHE = ["/offline", "/icons/icon-192.png", "/icons/icon-512.png"];
const CACHEABLE = [/^\/_next\/static\//, /^\/engine\//, /^\/pieces\//, /^\/icons\//, /^\/bots\//];
// Pages worth reopening offline (no sign-in, admin or account pages).
const PAGE_OK = [/^\/$/, /^\/play(\/|$)/, /^\/puzzles(\/|$)/, /^\/openings$/, /^\/learn$/];
const MAX_PAGES = 30;

self.addEventListener("install", (event) => {
  event.waitUntil(caches.open(STATIC).then((c) => c.addAll(PRECACHE)).then(() => self.skipWaiting()));
});

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("rookery-") && k !== STATIC && k !== PAGES).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  );
});

async function remember(req, res) {
  const cache = await caches.open(PAGES);
  await cache.put(req, res);
  const keys = await cache.keys();
  if (keys.length > MAX_PAGES) await cache.delete(keys[0]);
}

self.addEventListener("fetch", (event) => {
  const req = event.request;
  if (req.method !== "GET") return;
  const url = new URL(req.url);
  if (url.origin !== self.location.origin) return; // Supabase and everything else goes straight to the network

  if (req.mode === "navigate") {
    const cacheable = PAGE_OK.some((re) => re.test(url.pathname));
    event.respondWith(
      fetch(req)
        .then((res) => {
          if (cacheable && res.ok) remember(req, res.clone());
          return res;
        })
        .catch(async () => (cacheable ? (await caches.match(req)) : undefined) || (await caches.match("/offline"))),
    );
    return;
  }
  if (CACHEABLE.some((re) => re.test(url.pathname))) {
    event.respondWith(
      caches.match(req).then(
        (hit) =>
          hit ||
          fetch(req).then((res) => {
            if (res.ok) {
              const copy = res.clone();
              caches.open(STATIC).then((c) => c.put(req, copy));
            }
            return res;
          }),
      ),
    );
  }
});
