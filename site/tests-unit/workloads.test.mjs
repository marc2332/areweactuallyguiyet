import { test } from 'node:test';
import assert from 'node:assert/strict';
import { workloadSections } from '../src/lib/workloads.mjs';

const group = (scenario, scenarioVersion = 1) => ({ scenario, scenarioVersion, points: [], missing: [] });
const section = groups => workloadSections({ groups })[0];

test('only scrolling is currently configured for display', () => {
  const workloads = workloadSections({ groups: {} });
  assert.deepEqual(workloads.map(workload => workload.scenario), ['scroll']);
  assert.ok(workloads.every(workload => workload.label && workload.groups.length === 0));
});

test('scrolling prefers present-call cadence when captured', () => {
  const workload = section({ renderer_fps: [group('scroll')], cpu_ms: [group('scroll')] });
  assert.equal(workload.metric, 'renderer_fps');
});

test('scrolling falls back to labeled CPU measurements without cadence data', () => {
  const workload = section({ cpu_ms: [group('scroll')] });
  assert.equal(workload.metric, 'cpu_ms');
  assert.equal(workload.groups[0].scenario, 'scroll');
});

test('workload versions remain separate and unconfigured workloads are excluded', () => {
  const first = group('scroll', 1);
  const second = group('scroll', 2);
  const workload = section({ renderer_fps: [first, group('future-workload'), second] });
  assert.deepEqual(workload.groups, [first, second]);
});
