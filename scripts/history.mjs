import { readFile, writeFile } from 'node:fs/promises';

const statuses = new Set(['ok', 'unsupported', 'unavailable', 'failed', 'timed-out', 'invalid-output']);
const metricUnits = { startup_ms: 'ms', cpu_ms: 'ms', rss_mib: 'MiB', pss_mib: 'MiB', peak_rss_mib: 'MiB' };
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const exactKeys = (value, keys) => object(value) && Object.keys(value).sort().join(',') === [...keys].sort().join(',');

export function validateHistory(history) {
  if (!exactKeys(history, ['schemaVersion', 'runs']) || history.schemaVersion !== 1 || !Array.isArray(history.runs)) throw new Error('Expected history {schemaVersion:1,runs:[]}');
  const runIds = new Set();
  for (const run of history.runs) {
    if (!object(run) || !['id', 'collectedAt', 'machine', 'results', 'metadata'].every(key => key in run) || typeof run.id !== 'string' || !run.id || typeof run.collectedAt !== 'string' || !Number.isFinite(Date.parse(run.collectedAt))) throw new Error('Run requires id, collectedAt, machine, results and metadata');
    if (runIds.has(run.id)) throw new Error(`Duplicate run id ${run.id}`);
    runIds.add(run.id);
    if (!object(run.machine) || typeof run.machine.id !== 'string' || typeof run.machine.label !== 'string' || !['os', 'cpu', 'gpu'].every(key => run.machine[key] === null || typeof run.machine[key] === 'string')) throw new Error(`Invalid machine in ${run.id}`);
    if (!Array.isArray(run.results) || !object(run.metadata)) throw new Error(`Invalid results or metadata in ${run.id}`);
    const keys = new Set();
    for (const result of run.results) {
      if (!exactKeys(result, ['framework', 'scenario', 'scenarioVersion', 'status', 'reason', 'metrics', 'repetitions']) || typeof result.framework !== 'string' || typeof result.scenario !== 'string' || (!Number.isSafeInteger(result.scenarioVersion) || result.scenarioVersion < 1) || !statuses.has(result.status) || !(result.reason === null || typeof result.reason === 'string') || !object(result.metrics) || !Array.isArray(result.repetitions)) throw new Error(`Invalid result in ${run.id}`);
      const key = `${result.framework}/${result.scenario}`;
      if (keys.has(key)) throw new Error(`Duplicate result ${key} in ${run.id}`);
      keys.add(key);
      for (const [metric, sample] of Object.entries(result.metrics)) {
        if (!(metric in metricUnits) || !exactKeys(sample, ['value', 'unit']) || !Number.isFinite(sample.value) || sample.value < 0 || sample.unit !== metricUnits[metric]) throw new Error(`Invalid metric ${metric}`);
      }
      if (result.status !== 'ok' && Object.keys(result.metrics).length) throw new Error(`Non-ok result has metrics: ${key}`);
    }
  }
  return history;
}

export function chartHistory(history) {
  const metadataKeys = ['buildProfile', 'frameworkLabels', 'frameworkBaseIds', 'frameworkVersions', 'repetitions', 'durationSeconds', 'warmupSeconds', 'startupMemorySettleSeconds', 'switchMode'];
  return {
    schemaVersion: history.schemaVersion,
    runs: history.runs.map(run => ({
      id: run.id,
      collectedAt: run.collectedAt,
      machine: run.machine,
      metadata: run.results.some(result => result.scenario === 'hover')
        ? run.metadata
        : Object.fromEntries(metadataKeys.filter(key => key in run.metadata).map(key => [key, run.metadata[key]])),
      results: run.results.map(result => ({
        ...result,
        repetitions: result.repetitions.map(repetition => ({
          index: repetition.index,
          metrics: repetition.metrics ?? {},
          ...(result.scenario === 'hover' ? { startupBoundary: repetition.startupBoundary, events: repetition.events ?? [] } : {}),
        })),
      })),
    })),
  };
}

export function validateFullCollection(run, adapters) {
  const included = new Set(run.results.map(result => `${result.framework}/${result.scenario}`));
  const missing = adapters.flatMap(adapter => Object.keys(adapter.scenarios).map(scenario => `${adapter.id}/${scenario}`)).filter(key => !included.has(key));
  if (missing.length) throw new Error(`Only full collections can be published. Individual runs are temporary inputs. Missing ${missing.length} cases, including ${missing.slice(0, 3).join(', ')}.`);
}

export async function importRun(inputPath, historyPath = 'site/data/history.json', adapters) {
  const incoming = JSON.parse(await readFile(inputPath, 'utf8'));
  const history = validateHistory(JSON.parse(await readFile(historyPath, 'utf8')));
  if (!object(incoming) || incoming.schemaVersion !== 1 || !['id', 'collectedAt', 'machine', 'results', 'metadata'].every(key => key in incoming)) throw new Error('Input must be a schemaVersion 1 runner run object');
  const run = { ...incoming };
  delete run.schemaVersion;
  const next = { schemaVersion: 1, runs: [...history.runs.filter(item => item.id !== run.id), run] };
  validateHistory(next);
  adapters ??= JSON.parse(await readFile(new URL('../apps/catalog.json', import.meta.url), 'utf8'));
  validateFullCollection(run, adapters);
  await writeFile(historyPath, `${JSON.stringify(next, null, 2)}\n`);
  return next;
}
