// Diagnostic worker has no fetch handler and does not cache or intercept pages.
self.addEventListener('install', () => self.skipWaiting());
self.addEventListener('activate', event => event.waitUntil(self.clients.claim()));
