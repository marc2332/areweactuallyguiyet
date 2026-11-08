import { readFile } from 'node:fs/promises';
import { spawn } from 'node:child_process';

const [framework, scenario] = process.argv.slice(2);
const adapters = JSON.parse(await readFile('apps/catalog.json', 'utf8'));
const adapter = adapters.find(candidate => candidate.id === framework);
const capability = adapter?.scenarios[scenario];
if (!adapter || !capability) {
  console.error('Usage: node scripts/launch.mjs <framework> <startup|scroll|virtual-scroll|switches|memory>');
  process.exitCode = 1;
} else if (capability.status !== 'implemented') {
  console.error(`${framework}/${scenario}: ${capability.status}. ${capability.reason ?? ''}`);
  process.exitCode = 1;
} else {
  const environment = { ...process.env };
  delete environment.GUI_BENCH_ADDR;
  const application = spawn('cargo', [
    'run', '--locked', '--manifest-path', adapter.manifest,
    '--no-default-features', '--features', scenario,
  ], { stdio: 'inherit', env: environment });
  application.on('error', error => {
    console.error(error.message);
    process.exitCode = 1;
  });
  application.on('exit', (code, signal) => {
    process.exitCode = code ?? (signal ? 1 : 0);
  });
}
