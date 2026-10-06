import { spawnSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const commands = [
  ['cargo', ['fmt', '--all', '--check']],
  ['cargo', ['test', '--locked', '--workspace']],
  ['cargo', ['clippy', '--locked', '--workspace', '--all-targets', '--', '-D', 'warnings']],
  ['cargo', ['build', '--locked', '--release', '-p', 'bridge-probe', '--target', 'wasm32-unknown-unknown']],
  ['cargo', ['clippy', '--locked', '-p', 'bridge-probe', '-p', 'rustscript-sdk', '--target', 'wasm32-unknown-unknown', '--', '-D', 'warnings']],
  ['cargo', ['run', '--locked', '-p', 'native-probe', '--', 'target/wasm32-unknown-unknown/release/bridge_probe.wasm']],
  [process.execPath, ['--test', 'tests/fixture.test.mjs']],
];
const report = { product: 'Mantle', scope: 'portable-feasibility-only', recordedAt: new Date().toISOString(),
  platform: process.platform, architecture: process.arch, browserPageRuntimeVerified: false, appleBuildVerified: false, checks: [] };
mkdirSync(resolve(root, 'artifacts'), { recursive: true });
const reportPath = resolve(root, 'artifacts/portable-checks.json');
for (const [command, args] of commands) {
  console.log(`> ${command} ${args.join(' ')}`);
  const result = spawnSync(command, args, { cwd: root, encoding: 'utf8', maxBuffer: 8 * 1024 * 1024, shell: false });
  process.stdout.write(result.stdout ?? ''); process.stderr.write(result.stderr ?? '');
  const passed = !result.error && result.status === 0;
  report.checks.push({ command, args, passed, exitCode: result.status, error: result.error?.message,
    stdout: result.stdout, stderr: result.stderr });
  report.passed = passed && report.checks.length === commands.length;
  writeFileSync(reportPath, JSON.stringify(report, null, 2) + '\n');
  if (!passed) { console.error(`Failed. Partial report: ${reportPath}`); process.exit(1); }
}
console.log(`Portable checks passed. Gate A browser and Apple checks remain pending. Report: ${reportPath}`);
