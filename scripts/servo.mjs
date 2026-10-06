import { readFileSync, existsSync, mkdirSync, readdirSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const pin = JSON.parse(readFileSync(resolve(root, 'engine/servo.json'), 'utf8'));
if (!/^[a-f0-9]{40}$/.test(pin.revision) || pin.repository !== 'https://github.com/servo/servo.git' || pin.checkout !== 'vendor/servo') throw new Error('Invalid engine pin');
const checkout = resolve(root, pin.checkout);
function run(command, args, cwd = checkout, capture = false) {
  const result = spawnSync(command, args, { cwd, stdio: capture ? 'pipe' : 'inherit', encoding: 'utf8', shell: false });
  if (result.error || result.status !== 0) throw new Error(result.error?.message ?? `${command} exited ${result.status}: ${result.stderr ?? ''}`);
  return result.stdout?.trim();
}

const action = process.argv[2];
if (!['checkout', 'bootstrap', 'build', 'run'].includes(action)) throw new Error('Usage: node scripts/servo.mjs checkout|bootstrap|build|run [URL]');
if (action === 'checkout') {
  if (existsSync(checkout) && !existsSync(resolve(checkout, '.git'))) {
    if (readdirSync(checkout).length) throw new Error('Refusing to replace an existing non-Git directory');
  }
  mkdirSync(checkout, { recursive: true });
  if (!existsSync(resolve(checkout, '.git'))) {
    run('git', ['init']); run('git', ['remote', 'add', 'origin', pin.repository]);
  }
  if (run('git', ['remote', 'get-url', 'origin'], checkout, true) !== pin.repository) throw new Error('Existing checkout has another origin');
  if (run('git', ['status', '--porcelain'], checkout, true)) throw new Error('Preserve the existing engine changes before checkout');
  run('git', ['fetch', '--depth=1', 'origin', pin.revision]);
  run('git', ['checkout', '--detach', pin.revision]);
} else {
  if (process.platform !== 'darwin') throw new Error('The Apple feasibility commands must run on macOS');
  if (run('git', ['rev-parse', 'HEAD'], checkout, true) !== pin.revision) throw new Error('Wrong Servo revision');
  run('xcodebuild', ['-version']);
  if (action === 'run') {
    const url = process.argv[3] ?? 'http://127.0.0.1:8178/';
    if (!['http:', 'https:'].includes(new URL(url).protocol)) throw new Error('Expected an HTTP(S) URL');
    run('./mach', ['run', url]);
  } else run('./mach', [action]);
}
