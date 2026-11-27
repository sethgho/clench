// The clench service worker: precaches the app shell, engine and icons so
// the reader works offline after the first visit. The build fills in the
// cache name (versioned by the content of every file) and the file list.
const CACHE = "__CACHE__"
const FILES = __FILES__

self.addEventListener("install", (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(FILES))
      .then(() => self.skipWaiting()),
  )
})

self.addEventListener("activate", (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) => Promise.all(keys.filter((k) => k.startsWith("clench-") && k !== CACHE).map((k) => caches.delete(k))))
      .then(() => self.clients.claim()),
  )
})

self.addEventListener("fetch", (event) => {
  const request = event.request
  if (request.method !== "GET") return
  const url = new URL(request.url)
  // Pages the reader fetches for the user are never cached or served from here.
  if (url.origin !== self.location.origin || !url.pathname.startsWith(new URL(self.registration.scope).pathname)) return
  if (request.mode === "navigate") {
    event.respondWith(
      fetch(request).catch(() => caches.open(CACHE).then((cache) => cache.match("./", { ignoreSearch: true }))),
    )
    return
  }
  event.respondWith(caches.match(request, { ignoreSearch: true }).then((hit) => hit || fetch(request)))
})
