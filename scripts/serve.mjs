import { createServer } from 'node:http';
import { createServer as createHttpsServer } from 'node:https';
import { readFile, stat } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const routes = new Map([
  ['/', ['web/index.html', 'text/html; charset=utf-8']],
  ['/probe.mjs', ['web/probe.mjs', 'text/javascript; charset=utf-8']],
  ['/worker.mjs', ['web/worker.mjs', 'text/javascript; charset=utf-8']],
  ['/service-worker.mjs', ['web/service-worker.mjs', 'text/javascript; charset=utf-8']],
  ['/bridge-probe.wasm', ['target/wasm32-unknown-unknown/release/bridge_probe.wasm', 'application/wasm']],
]);

export function createProbeServer(tls) {
  const factory = tls ? handler => createHttpsServer(tls, handler) : createServer;
  return factory(async (request, response) => {
    // Explicit routes prevent exposing source, engine checkout or local secrets.
    response.setHeader('Cache-Control', 'no-store');
    response.setHeader('X-Content-Type-Options', 'nosniff');
    response.setHeader('Content-Security-Policy', "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; worker-src 'self'; style-src 'unsafe-inline'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'");
    if (request.method !== 'GET' && request.method !== 'HEAD') {
      response.writeHead(405, { Allow: 'GET, HEAD' }).end(); return;
    }
    const route = routes.get(new URL(request.url, 'http://localhost').pathname);
    if (!route) { response.writeHead(404).end('Not found'); return; }
    try {
      const path = resolve(root, route[0]);
      const info = await stat(path);
      response.writeHead(200, { 'Content-Type': route[1], 'Content-Length': info.size });
      if (request.method === 'HEAD') response.end();
      else response.end(await readFile(path));
    } catch {
      response.writeHead(503, { 'Content-Type': 'text/plain' }).end('Fixture unavailable. Build the WASM probe first.');
    }
  });
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const port = Number(process.env.PORT ?? 8178);
  if (!Number.isInteger(port) || port < 1 || port > 65535) throw new Error('Invalid PORT');
  const cert = process.env.MANTLE_TLS_CERT;
  const key = process.env.MANTLE_TLS_KEY;
  if (Boolean(cert) !== Boolean(key)) throw new Error('Set both MANTLE_TLS_CERT and MANTLE_TLS_KEY');
  const tls = cert ? { cert: await readFile(cert), key: await readFile(key) } : undefined;
  createProbeServer(tls).listen(port, '127.0.0.1', () => console.log(`Mantle feasibility: ${tls ? 'https' : 'http'}://127.0.0.1:${port}/`));
}
