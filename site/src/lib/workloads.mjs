import { scenarioLabels } from './chart-data.mjs';

const definitions = [
  { scenario: 'scroll', metrics: ['renderer_fps', 'cpu_ms'] },
  { scenario: 'dashboard', metrics: ['dashboard_worst_gap_ms'] },
  { scenario: 'dashboard', metrics: ['dashboard_rss_mean_mib'] },
];

export function workloadSections(run) {
  const scaling = (run.groups.renderer_fps ?? []).some(group => group.rows);
  const workloads = scaling
    ? [definitions[0], { scenario: 'scroll', metrics: ['frame_gap_p95_ms'] }, ...definitions.slice(1)]
    : definitions;
  return workloads.map(({ scenario, metrics }) => {
    const matches = group => scenario === 'scroll' && scaling ? Boolean(group.rows) : group.scenario === scenario;
    const metric = metrics.find(candidate => (run.groups[candidate] ?? []).some(matches)) ?? metrics[0];
    return {
      scenario,
      label: scenario === 'scroll' && scaling ? 'Scroll scaling' : scenarioLabels[scenario],
      metric,
      scaling: scenario === 'scroll' && scaling,
      groups: (run.groups[metric] ?? []).filter(matches).sort((first, second) => (first.rows ?? 0) - (second.rows ?? 0)),
    };
  });
}
