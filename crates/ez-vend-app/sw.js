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
                    if (event.request.mode === 'navigate') {
                        return caches.match('./').then(function (fallback) {
                            return fallback || new Response('Offline', { status: 503 });
                        });
                    }
                    return new Response('Offline', { status: 503 });
                });
        })
    );
});
