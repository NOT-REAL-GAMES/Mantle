export const requiredImports = ['abi_version', 'capabilities', 'probe_add'];

export async function probeNativeImports(module, host) {
  const imports = WebAssembly.Module.imports(module);
  if (imports.length !== 3 || new Set(imports.map(i => i.name)).size !== 3 || imports.some(i => i.module !== 'rustscript_v1' || i.kind !== 'function' || !requiredImports.includes(i.name))) {
    throw new Error('Guest import contract mismatch');
  }
  if (!host || requiredImports.some(name => typeof host[name] !== 'function')) {
    return { status: 'unavailable', browserPageRuntimeVerified: false, reason: 'No engine-provided Rustscript imports; no fallback installed' };
  }
  // Pass native function references directly. Never replace these with JS closures.
  const instance = await WebAssembly.instantiate(module, { rustscript_v1: {
    abi_version: host.abi_version,
    capabilities: host.capabilities,
    probe_add: host.probe_add,
  } });
  const result = instance.exports.run_probe();
  return { status: result === 42 ? 'candidate' : 'failed', result,
    noJavaScriptAdapterInFixture: true, browserPageRuntimeVerified: false,
    reason: 'A candidate result needs independently verified native engine counters/call-stack evidence' };
}

async function storageProbe() {
  const name = `mantle-feasibility-${crypto.randomUUID()}`;
  let db;
  try {
    db = await new Promise((resolve, reject) => {
      const request = indexedDB.open(name, 1);
      request.onupgradeneeded = () => request.result.createObjectStore('probe');
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    await new Promise((resolve, reject) => {
      const transaction = db.transaction('probe', 'readwrite');
      transaction.objectStore('probe').put(new Uint8Array([42]), 'value');
      transaction.oncomplete = resolve;
      transaction.onerror = () => reject(transaction.error);
      transaction.onabort = () => reject(transaction.error ?? new Error('Storage aborted'));
    });
    const value = await new Promise((resolve, reject) => {
      const request = db.transaction('probe').objectStore('probe').get('value');
      request.onsuccess = () => resolve(request.result);
      request.onerror = () => reject(request.error);
    });
    if (value?.[0] !== 42) throw new Error('IndexedDB round trip failed');
    return { status: 'passed' };
  } finally {
    db?.close();
    await new Promise((resolve, reject) => {
      const request = indexedDB.deleteDatabase(name);
      request.onsuccess = resolve;
      request.onerror = () => reject(request.error);
      request.onblocked = () => reject(new Error('Diagnostic database cleanup blocked'));
    });
  }
}

async function workerProbe() {
  const worker = new Worker('/worker.mjs', { type: 'module' });
  try {
    const data = new Uint8Array([42]);
    await new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('Worker timed out')), 5000);
      worker.onmessage = event => { clearTimeout(timeout); event.data[0] === 42 ? resolve() : reject(new Error('Worker data mismatch')); };
      worker.onerror = event => { clearTimeout(timeout); reject(new Error(event.message)); };
      worker.postMessage(data, [data.buffer]);
      if (data.byteLength !== 0) { clearTimeout(timeout); reject(new Error('Buffer did not transfer')); }
    });
    return { status: 'passed' };
  } finally { worker.terminate(); }
}

let gpuResources;
async function gpuProbe() {
  gpuResources?.context.unconfigure(); gpuResources?.device.destroy(); gpuResources = undefined;
  if (!navigator.gpu) throw new Error('navigator.gpu unavailable');
  const adapter = await navigator.gpu.requestAdapter();
  if (!adapter) throw new Error('No WebGPU adapter');
  const device = await adapter.requestDevice();
  const canvas = document.getElementById('gpu');
  const context = canvas.getContext('webgpu');
  try {
    if (!context) throw new Error('No WebGPU canvas context');
    device.pushErrorScope('validation');
    context.configure({ device, format: navigator.gpu.getPreferredCanvasFormat(), alphaMode: 'opaque' });
    const encoder = device.createCommandEncoder();
    const pass = encoder.beginRenderPass({ colorAttachments: [{ view: context.getCurrentTexture().createView(), clearValue: { r: .1, g: .7, b: .5, a: 1 }, loadOp: 'clear', storeOp: 'store' }] });
    pass.end();
    device.queue.submit([encoder.finish()]);
    await device.queue.onSubmittedWorkDone();
    const error = await device.popErrorScope();
    if (error) throw new Error(error.message);
    gpuResources = { context, device };
    return { status: 'passed', visualCheck: 'Confirm mint rectangle', physicalPresentationVerified: false,
      limits: { maxBufferSize: adapter.limits.maxBufferSize, maxTextureDimension2D: adapter.limits.maxTextureDimension2D } };
  } catch (error) { context?.unconfigure(); device.destroy(); throw error; }
}

async function serviceWorkerProbe() {
  if (!('serviceWorker' in navigator)) throw new Error('Service worker unavailable');
  let registration;
  try {
    registration = await navigator.serviceWorker.register('/service-worker.mjs', { scope: '/' });
    const ready = await Promise.race([navigator.serviceWorker.ready, new Promise((_, reject) => setTimeout(() => reject(new Error('Service worker timed out')), 5000))]);
    if (ready.scope !== new URL('/', location.href).href) throw new Error('Unexpected service worker scope');
    return { status: 'passed' };
  } finally { if (registration) await registration.unregister(); }
}

if (typeof document !== 'undefined') {
  window.addEventListener('pagehide', () => { gpuResources?.context.unconfigure(); gpuResources?.device.destroy(); gpuResources = undefined; });
  let lastReport;
  const output = document.getElementById('report');
  const run = document.getElementById('run');
  const download = document.getElementById('download');
  run.onclick = async () => {
    run.disabled = true; download.disabled = true;
    const report = { product: 'Mantle', recordedAt: new Date().toISOString(), userAgent: navigator.userAgent,
      origin: location.origin, secureContext: isSecureContext, evidenceScope: 'browser-fixture', checks: {} };
    const checks = { webgpu: gpuProbe, indexedDB: storageProbe, worker: workerProbe, serviceWorker: serviceWorkerProbe,
      rustscript: async () => {
        const response = await fetch('/bridge-probe.wasm');
        if (!response.ok) throw new Error('Build the WASM probe first');
        const module = await WebAssembly.compile(await response.arrayBuffer());
        return probeNativeImports(module, navigator.rustscript);
      } };
    for (const [name, check] of Object.entries(checks)) {
      report.checks[name] = { status: 'running' }; output.textContent = JSON.stringify(report, null, 2);
      try { report.checks[name] = await check(); }
      catch (error) { report.checks[name] = { status: 'failed', error: String(error) }; }
    }
    lastReport = report; output.textContent = JSON.stringify(report, null, 2);
    run.disabled = false; download.disabled = false;
  };
  download.onclick = () => {
    const url = URL.createObjectURL(new Blob([JSON.stringify(lastReport, null, 2)], { type: 'application/json' }));
    const link = document.createElement('a'); link.href = url; link.download = 'mantle-feasibility.json'; link.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
  };
}
