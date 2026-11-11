import { test } from 'node:test';
import assert from 'node:assert/strict';
import { scrollSeries } from '../src/lib/scroll-scaling.mjs';
import { groupMetricResults } from '../src/lib/chart-data.mjs';

const point = (framework, value) => ({ framework, value, minimum: value - 1, maximum: value + 1 });
const group = (rows, points, boundary = 'mangohud-present-observation') => ({ rows, points, boundary, scenarioVersion: 3 });

test('scaling orders samples numerically and leaves missing points disconnected', () => {
  const data = scrollSeries([
    group(7000, [point('freya', 60)]),
    group(1000, [point('freya', 144), point('egui', 140)]),
    group(3000, [point('egui', 100)]),
  ]);
  assert.deepEqual(data.rows, [1000, 3000, 7000]);
  assert.deepEqual(data.series[0].samples.map(sample => sample.point?.value ?? null), [144, null, 60]);
  assert.equal(data.series[0].samples[0].point.minimum, 143);
});

test('hidden frameworks and incompatible boundaries are not combined', () => {
  const data = scrollSeries([
    group(1000, [point('freya', 144), point('egui', 140)]),
    group(3000, [point('freya', 80)], 'present-returned'),
  ], new Set(['egui']));
  assert.equal(data.series.length, 2);
  assert.ok(data.series.every(series => series.framework === 'freya'));
  assert.deepEqual(data.series.map(series => series.samples.map(sample => sample.point?.value ?? null)), [[144, null], [null, 80]]);
});

test('chart groups retain row counts and repetition ranges without mixing legacy data', () => {
  const result = scenario => ({
    framework: 'freya', scenario, scenarioVersion: 3, status: 'ok', reason: null,
    metrics: { renderer_fps: { value: 100, unit: 'frames/s' } },
    repetitions: [90, 110].map(value => ({
      metrics: { renderer_fps: { value, unit: 'frames/s' } },
      events: [{ event: 'mangohud-capture', trace: { boundary: 'mangohud-present-observation' } }],
    })),
  });
  const failed = { ...result('scroll-7000'), status: 'failed', reason: 'Capture failed', metrics: {}, repetitions: [] };
  const groups = groupMetricResults({ results: [result('scroll'), result('scroll-3000'), failed] }, 'renderer_fps');
  assert.equal(groups[0].rows, undefined);
  assert.equal(groups[1].rows, 3000);
  assert.equal(groups[1].label, 'Scroll 3,000 rows');
  assert.equal(groups[1].points[0].minimum, 90);
  assert.equal(groups[1].points[0].maximum, 110);
  assert.equal(groups[2].rows, 7000);
  assert.equal(groups[2].points.length, 0);
  assert.equal(groups[2].missing[0].reason, 'Capture failed');
  assert.deepEqual(scrollSeries(groups.filter(group => group.rows)).series[0].samples.map(sample => sample.point?.value ?? null), [100, null]);
});
