import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { createProbeServer } from '../scripts/serve.mjs';
import { probeNativeImports, requiredImports } from '../web/probe.mjs';

const bytes = await readFile(new URL('../target/wasm32-unknown-unknown/release/bridge_probe.wasm', import.meta.url));
const module = await WebAssembly.compile(bytes);

test('actual Rust guest has exactly the native diagnostic ABI and explicit failures', async () => {
  const imports = WebAssembly.Module.imports(module);
  assert.equal(imports.length, 3);
  assert.deepEqual(imports.map(i => i.name).sort(), [...requiredImports].sort());
  assert.ok(imports.every(i => i.module === 'rustscript_v1' && i.kind === 'function'));
  const missing = await probeNativeImports(module, undefined);
  assert.equal(missing.status, 'unavailable');
  assert.equal(missing.browserPageRuntimeVerified, false);
  await assert.rejects(WebAssembly.instantiate(module, {}));
  for (const [abi, caps, add, expected] of [[2, 1, 42, -2], [1, 0, 42, -3], [1, 1, 41, -4]]) {
    const result = await probeNativeImports(module, { abi_version: () => abi, capabilities: () => caps, probe_add: () => add });
    assert.equal(result.result, expected);
    assert.equal(result.status, 'failed');
  }
});

test('a JS mock cannot establish native browser evidence', async () => {
  const result = await probeNativeImports(module, { abi_version: () => 1, capabilities: () => 1, probe_add: (a, b) => a + b });
  assert.equal(result.result, 42);
  assert.equal(result.status, 'candidate');
  assert.equal(result.browserPageRuntimeVerified, false);
});

test('server exposes only fixture files and WASM, with no cache and CSP', async t => {
  const server = createProbeServer();
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  t.after(() => new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve())));
  const base = `http://127.0.0.1:${server.address().port}`;
  const wasm = await fetch(`${base}/bridge-probe.wasm`);
  assert.equal(wasm.status, 200);
  assert.equal(wasm.headers.get('content-type'), 'application/wasm');
  assert.equal(wasm.headers.get('cache-control'), 'no-store');
  assert.match(wasm.headers.get('content-security-policy'), /wasm-unsafe-eval/);
  const head = await fetch(base, { method: 'HEAD' });
  assert.equal(head.status, 200);
  assert.equal(await head.text(), '');
  for (const path of ['/Cargo.toml', '/.git/config', '/vendor/servo/Cargo.toml', '/../../.secrets.env', '/%2e%2e/package.json']) {
    assert.equal((await fetch(base + path)).status, 404);
  }
  assert.equal((await fetch(base, { method: 'POST' })).status, 405);
});
