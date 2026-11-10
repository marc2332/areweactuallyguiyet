import { readFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { validateFeatures, validateLibraries } from './features.mjs';
import { validateHistory } from './history.mjs';
import { collectionDetails, frameworkColor, frameworkVersionsForScenario, groupMetricResults, metricLabels, responsivenessDetails } from './chart-data.mjs';

const load = async name => JSON.parse(await readFile(resolve(process.cwd(), 'data', `${name}.json`), 'utf8'));
export const { libraries } = validateLibraries(await load('libraries'));
export const features = validateFeatures(await load('features'), libraries);
const history = validateHistory(await load('history'));
const baseNames = Object.fromEntries(libraries.map(library => [library.id, library.name]));

export const metrics = {
  renderer_fps: { ...metricLabels.renderer_fps, note: 'Closer to 144 FPS is better, matching my 144 Hz screen.' },
  frame_gap_max_ms: { ...metricLabels.frame_gap_max_ms, note: 'Includes silent periods at both ends of the workload. Lower is better, but does not establish display latency.' },
  cpu_ms: { ...metricLabels.cpu_ms, note: 'CPU consumed during the workload. Less CPU can mean less work completed, not a faster UI.' },
  pss_mib: { ...metricLabels.pss_mib, note: 'Process-tree proportional memory. Shared pages are apportioned between processes.' },
  frame_gap_p95_ms: { ...metricLabels.frame_gap_p95_ms, note: 'Gaps between present calls, not confirmed display latency. Also inspect the longest gap.' },
  frame_gap_p99_ms: { ...metricLabels.frame_gap_p99_ms, note: 'Gaps between present calls, not confirmed display latency. Also inspect the longest gap.' },
  rss_mib: { ...metricLabels.rss_mib, note: 'Resident memory at workload end. Shared pages may be counted more than once.' },
};

export const dashboard = {
  metrics,
  runs: [...history.runs]
    .sort((first, second) => Date.parse(second.collectedAt) - Date.parse(first.collectedAt) || second.id.localeCompare(first.id))
    .map(run => {
      for (const result of run.results) {
        const base = run.metadata.frameworkBaseIds?.[result.framework] ?? result.framework;
        if (!libraries.some(library => library.id === base)) throw new Error(`Unknown framework ${result.framework}`);
        if (base !== result.framework && !run.metadata.frameworkLabels?.[result.framework]?.trim()) throw new Error(`Unlabeled variant ${result.framework}`);
      }
      const names = { ...baseNames, ...run.metadata.frameworkLabels };
      return {
        id: run.id,
        collectedAt: run.collectedAt,
        measurement: run.metadata.measurement ?? 'legacy-renderer-traces',
        machine: run.machine,
        profile: run.metadata.buildProfile ?? 'Unknown profile',
        timing: collectionDetails(run).timing,
        responsiveness: responsivenessDetails(run),
        frameworks: [...new Set(run.results.map(result => result.framework))].map(id => ({ id, name: names[id] ?? id, color: frameworkColor(id) })),
        groups: Object.fromEntries(Object.keys(metrics).map(metric => [metric, groupMetricResults(run, metric).map(group => ({
          ...group,
          versions: frameworkVersionsForScenario(run, group.scenario),
        }))])),
      };
    }),
};
