const CACHE_VERSION = '__CACHE_VERSION__';
const CACHE_NAME = 'ez-vend-' + CACHE_VERSION;

self.addEventListener('install', function (event) {
    event.waitUntil(self.skipWaiting());
});

self.addEventListener('activate', function (event) {
    event.waitUntil(
        caches
            .keys()
            .then(function (keys) {
                return Promise.all(
                    keys
                        .filter(function (key) {
                            return key.startsWith('ez-vend-') && key !== CACHE_NAME;
                        })
                        .map(function (key) {
                            return caches.delete(key);
                        })
                );
            })
            .then(function () {
                return self.clients.claim();
            })
    );
});

self.addEventListener('fetch', function (event) {
    if (event.request.method !== 'GET') return;
    if (!event.request.url.startsWith(self.location.origin)) return;

    // Navigation requests (the HTML document itself) must be network-first:
    // URLs vary per route (e.g. /booths/{id}/report) and are not content-hashed,
    // so a cache-first navigate would permanently freeze a stale index.html
    // under that URL after any deploy. Hashed assets (css/js/wasm) stay
    // cache-first below since their URL changes whenever content changes.
    if (event.request.mode === 'navigate') {
        event.respondWith(
            fetch(event.request)
                .then(function (response) {
                    if (response && response.status === 200) {
                        var toCache = response.clone();
                        caches.open(CACHE_NAME).then(function (cache) {
                            cache.put(event.request, toCache);
                        });
                    }
                    return response;
                })
                .catch(function () {
                    return caches.match(event.request).then(function (cached) {
                        if (cached) return cached;
                        return caches.match('./').then(function (fallback) {
                            return fallback || new Response('Offline', { status: 503 });
                        });
                    });
                })
        );
        return;
    }

    event.respondWith(
        caches.match(event.request).then(function (cached) {
            if (cached) return cached;

            return fetch(event.request)
                .then(function (response) {
                    if (!response || response.status !== 200 || response.type === 'opaque') {
                        return response || new Response('Offline', { status: 503 });
                    }
                    var toCache = response.clone();
                    caches.open(CACHE_NAME).then(function (cache) {
                        cache.put(event.request, toCache);
                    });
                    return response;
                })
                .catch(function () {
                    return new Response('Offline', { status: 503 });
                });
        })
    );
});
