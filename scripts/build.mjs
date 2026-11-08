import { cp, mkdir, readFile, rm, stat, writeFile } from 'node:fs/promises';
import { execFileSync } from 'node:child_process';
import { validateHistory, chartHistory } from './history.mjs';
import { validateFeatures, validateLibraries } from './features.mjs';

const { libraries } = validateLibraries(JSON.parse(await readFile('site/data/libraries.json', 'utf8')));
const features = validateFeatures(JSON.parse(await readFile('site/data/features.json', 'utf8')), libraries);
const history = validateHistory(JSON.parse(await readFile('site/data/history.json', 'utf8')));
for (const run of history.runs) {
  for (const result of run.results) {
    const baseIdentifier = run.metadata.frameworkBaseIds?.[result.framework] ?? result.framework;
    const knownBase = libraries.some(library => library.id === baseIdentifier);
    const labeledVariant = baseIdentifier === result.framework || typeof run.metadata.frameworkLabels?.[result.framework] === 'string' && Boolean(run.metadata.frameworkLabels[result.framework].trim());
    if (!knownBase || !labeledVariant) throw new Error(`Unknown result framework ${result.framework}`);
  }
}
for (const path of ['site/app.js', 'site/benchmarks.js', 'site/chart-data.mjs']) {
  execFileSync(process.execPath, ['--check', path], { stdio: 'inherit' });
}
const html = await readFile('site/index.html', 'utf8');
for (const match of html.matchAll(/(?:href|src)="([^"#]+)"/g)) {
  const path = match[1];
  if (path.includes(':') || path.startsWith('{{')) continue;
  await stat(`site/${path}`);
}
await rm('dist', { recursive: true, force: true });
await mkdir('dist', { recursive: true });
await cp('site', 'dist', { recursive: true });
await writeFile('dist/data/benchmark-runs.json', `${JSON.stringify(chartHistory(history))}\n`);
const generated = new Date().toISOString();
await writeFile('dist/index.html', html.replaceAll('{{generated-at}}', generated).replaceAll('{{generated-date}}', generated.slice(0, 10)));
console.log(`Built static site with ${libraries.length} libraries, ${features.claims.length} reviewed-or-unknown claims, and ${history.runs.length} runs.`);
