import { test } from 'node:test';
import assert from 'node:assert/strict';
import { workloadSections } from '../src/lib/workloads.mjs';

const group = (scenario, scenarioVersion = 1) => ({ scenario, scenarioVersion, points: [], missing: [] });
const section = groups => workloadSections({ groups })[0];

test('scrolling and dashboard workloads are configured for display', () => {
  const workloads = workloadSections({ groups: {} });
  assert.deepEqual(workloads.map(({ scenario, metric }) => ({ scenario, metric })), [
    { scenario: 'scroll', metric: 'renderer_fps' },
    { scenario: 'dashboard', metric: 'dashboard_worst_gap_ms' },
    { scenario: 'dashboard', metric: 'dashboard_rss_mean_mib' },
  ]);
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

test('dashboard uses the worst full-window gap instead of median intervals', () => {
  const workloads = workloadSections({ groups: {
    dashboard_present_median_ms: [group('dashboard')],
    dashboard_worst_gap_ms: [group('dashboard')],
  } });
  assert.equal(workloads[1].metric, 'dashboard_worst_gap_ms');
  assert.equal(workloads[1].label, 'Dashboard interaction');
});

test('dashboard memory remains separate from responsiveness and excludes other workloads', () => {
  const responsiveness = group('dashboard');
  const memory = group('dashboard');
  const workloads = workloadSections({ groups: {
    dashboard_worst_gap_ms: [responsiveness],
    dashboard_rss_mean_mib: [memory, group('scroll')],
  } });
  assert.deepEqual(workloads[1].groups, [responsiveness]);
  assert.equal(workloads[2].scenario, 'dashboard');
  assert.equal(workloads[2].metric, 'dashboard_rss_mean_mib');
  assert.equal(workloads[2].label, 'Dashboard interaction');
  assert.deepEqual(workloads[2].groups, [memory]);
});

test('workload versions remain separate and unconfigured workloads are excluded', () => {
  const first = group('scroll', 1);
  const second = group('scroll', 2);
  const workload = section({ renderer_fps: [first, group('future-workload'), second] });
  assert.deepEqual(workload.groups, [first, second]);
});
