import { scenarioLabels } from './chart-data.mjs';

const definitions = [
  { scenario: 'scroll', metrics: ['renderer_fps', 'cpu_ms'] },
];

export function workloadSections(run) {
  return definitions.map(({ scenario, metrics }) => {
    const metric = metrics.find(candidate => (run.groups[candidate] ?? []).some(group => group.scenario === scenario)) ?? metrics[0];
    return {
      scenario,
      label: scenarioLabels[scenario],
      metric,
      groups: (run.groups[metric] ?? []).filter(group => group.scenario === scenario),
    };
  });
}
