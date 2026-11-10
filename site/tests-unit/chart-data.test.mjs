import { test } from 'node:test';
import assert from 'node:assert/strict';
import { validateHistory, chartHistory } from '../src/lib/history.mjs';
import { collectionDetails, frameworkVersionsForScenario, groupMetricResults, responsivenessDetails } from '../src/lib/chart-data.mjs';

function history() {
  return {
    schemaVersion: 1,
    runs: [{
      id: 'pilot', collectedAt: '2026-11-10T12:00:00Z',
      machine: { id: 'desktop', label: 'Desktop', os: 'Linux', cpu: null, gpu: null },
      metadata: { responsivenessVersion: 1, responsivenessScope: 'Renderer cadence, not displayed content' },
      results: [{ framework: 'freya', scenario: 'scroll', scenarioVersion: 1, status: 'ok', reason: null,
        metrics: {
          renderer_fps: { value: 0, unit: 'frames/s' },
          frame_gap_max_ms: { value: 15000, unit: 'ms' },
        },
        repetitions: [{ index: 0, metrics: {}, events: [{ event: 'frame-activity', trace: { boundary: 'present-returned', timestamps_ns: [] } }] }],
      }],
    }],
  };
}

test('renderer evidence survives chart history export for scrolling', () => {
  const data = validateHistory(history());
  const exported = chartHistory(data);
  assert.equal(exported.runs[0].metadata.responsivenessVersion, 1);
  assert.deepEqual(exported.runs[0].results[0].repetitions[0].events, data.runs[0].results[0].repetitions[0].events);
});

test('zero renderer activity remains visible instead of disappearing from the comparison', () => {
  const data = validateHistory(history());
  const groups = groupMetricResults(data.runs[0], 'renderer_fps');
  assert.equal(groups.length, 1);
  assert.equal(groups[0].points[0].value, 0);
});

test('only present calls appear in renderer charts and earlier stages remain explicitly unavailable', () => {
  const data = history();
  const first = data.runs[0].results[0];
  first.repetitions[0].events[0].trace.boundary = 'present-returned';
  const second = structuredClone(first);
  second.framework = 'rune';
  second.repetitions[0].events[0].trace.boundary = 'frame-encoded';
  data.runs[0].results.push(second);
  const groups = groupMetricResults(data.runs[0], 'renderer_fps');
  assert.equal(groups.length, 1);
  assert.equal(groups[0].boundary, 'present-returned');
  assert.equal(groups[0].points.length, 1);
  assert.equal(groups[0].missing[0].framework, 'rune');
  assert.match(groups[0].missing[0].reason, /No compatible present-returned capture/);
  assert.equal(data.runs[0].results[1].status, 'ok');
});

test('MangoHud observations are separated from present-return captures', () => {
  const run = history().runs[0];
  const external = structuredClone(run.results[0]);
  external.framework = 'whisker';
  external.repetitions[0].events = [{ event: 'mangohud-capture', trace: { boundary: 'mangohud-present-observation', csv: '/capture.csv' } }];
  run.results.push(external);
  const groups = groupMetricResults(run, 'renderer_fps');
  assert.deepEqual(groups.map(group => group.boundary), ['present-returned', 'mangohud-present-observation']);
  assert.deepEqual(groups[0].points.map(point => point.framework), ['freya']);
  assert.deepEqual(groups[1].points.map(point => point.framework), ['whisker']);
});

test('a scroll-only refresh retains traces without claiming other workloads were remeasured', () => {
  const data = history();
  const run = data.runs[0];
  delete run.metadata.responsivenessVersion;
  run.metadata.durationSeconds = 5;
  run.metadata.repetitions = 3;
  run.metadata.warmupSeconds = 2;
  run.metadata.frameworkVersions = { freya: 'inherited' };
  run.metadata.scenarioCollections = { scroll: {
    durationSeconds: 15, repetitions: 3, warmupSeconds: 2, responsivenessVersion: 2,
    frameworkVersions: { freya: 'refreshed' },
    displayRefreshProbe: { activeModes: ['   2560x1440    143.91*+'] },
  } };
  const exported = chartHistory(data);
  assert.deepEqual(exported.runs[0].results[0].repetitions[0].events, run.results[0].repetitions[0].events);
  assert.equal(exported.runs[0].metadata.scenarioCollections.scroll.durationSeconds, 15);
  assert.equal(frameworkVersionsForScenario(run, 'scroll').freya, 'refreshed');
  assert.equal(frameworkVersionsForScenario(run, 'future-workload').freya, 'inherited');
  assert.match(collectionDetails(run).timing, /5s runs/);
  assert.match(collectionDetails(run).timing, /Scroll 2,000 rows: 15s × 3/);
  assert.match(collectionDetails(run).timing, /monitor mode 143\.91Hz/);
  assert.match(responsivenessDetails(run), /other workloads inherited/);
});

test('history format accepts future workload identifiers without a schema change', () => {
  const data = history();
  data.runs[0].results[0].scenario = 'future-workload';
  assert.equal(validateHistory(data), data);
});

test('dashboard interval metrics are validated and use dashboard-specific collection metadata', () => {
  const data = history();
  const run = data.runs[0];
  run.metadata.scenarioCollections = { dashboard: { repetitions: 2, actionDelaySeconds: 0.15, frameworkVersions: { freya: 'dashboard version' } } };
  assert.match(collectionDetails(run).timing, /Dashboard interaction: fixed actions × 2 · 0.15s between actions/);
  run.results.push({ framework: 'freya', scenario: 'dashboard', scenarioVersion: 1, status: 'ok', reason: null,
    metrics: { dashboard_present_median_ms: { value: 20, unit: 'ms' }, dashboard_worst_gap_ms: { value: 1200, unit: 'ms' } }, repetitions: [{ index: 0, metrics: {
      dashboard_present_median_ms: { value: 20, unit: 'ms' },
    } }] });
  assert.equal(validateHistory(data), data);
  assert.equal(frameworkVersionsForScenario(run, 'dashboard').freya, 'dashboard version');
  assert.equal(groupMetricResults(run, 'dashboard_present_median_ms')[0].points[0].value, 20);
  assert.equal(groupMetricResults(run, 'dashboard_worst_gap_ms')[0].points[0].value, 1200);
  run.results[1].metrics.dashboard_present_median_ms.unit = 'frames/s';
  assert.throws(() => validateHistory(data), /Invalid metric/);
});

test('dashboard bursts show their eight-second workload', () => {
  const run = history().runs[0];
  run.metadata.scenarioCollections = { dashboard: { measurement: 'mangohud-dashboard-v2', repetitions: 2 } };
  assert.match(collectionDetails(run).timing, /Dashboard interaction: 8s × 2 · four 2s phases/);
});

test('metric units are validated for renderer measurements', () => {
  const data = history();
  data.runs[0].results[0].metrics.renderer_fps.unit = 'ms';
  assert.throws(() => validateHistory(data), /Invalid metric/);
});
