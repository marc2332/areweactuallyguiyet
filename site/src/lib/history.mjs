const statuses = new Set(['ok', 'unsupported', 'unavailable', 'failed', 'timed-out', 'invalid-output']);
const metricUnits = { startup_ms: 'ms', cpu_ms: 'ms', rss_mib: 'MiB', pss_mib: 'MiB', peak_rss_mib: 'MiB', renderer_frames: 'frames', renderer_fps: 'frames/s', frame_gap_p95_ms: 'ms', frame_gap_p99_ms: 'ms', frame_gap_max_ms: 'ms' };
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
  const metadataKeys = ['buildProfile', 'frameworkLabels', 'frameworkBaseIds', 'frameworkVersions', 'repetitions', 'durationSeconds', 'warmupSeconds', 'startupMemorySettleSeconds', 'responsivenessVersion', 'responsivenessScope', 'scenarioCollections'];
  return {
    schemaVersion: history.schemaVersion,
    runs: history.runs.map(run => ({
      id: run.id,
      collectedAt: run.collectedAt,
      machine: run.machine,
      metadata: Object.fromEntries(metadataKeys.filter(key => key in run.metadata).map(key => [key, run.metadata[key]])),
      results: run.results.map(result => ({
        ...result,
        repetitions: result.repetitions.map(repetition => ({
          index: repetition.index,
          metrics: repetition.metrics ?? {},
          ...(run.metadata.responsivenessVersion || run.metadata.scenarioCollections?.[result.scenario]?.responsivenessVersion ? { startupBoundary: repetition.startupBoundary, events: repetition.events ?? [] } : {}),
        })),
      })),
    })),
  };
}

