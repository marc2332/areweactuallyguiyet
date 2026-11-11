import { test } from 'node:test';
import assert from 'node:assert/strict';
import { binarySizeChart } from '../src/lib/binary-sizes.mjs';

const snapshot = () => ({
  schemaVersion: 1,
  collectedAt: '2026-11-10T12:00:00Z',
  platform: { os: 'Linux', architecture: 'x86_64' },
  profile: 'production',
  method: 'As built',
  results: [{ framework: 'freya', name: 'Freya', binary: 'freya-scroll', bytes: 1572864 }],
});

test('binary sizes convert bytes to MiB without inventing repetitions', () => {
  const chart = binarySizeChart(snapshot());
  assert.equal(chart.group.points[0].value, 1.5);
  assert.equal(chart.group.points[0].minimum, 1.5);
  assert.equal(chart.group.points[0].maximum, 1.5);
  assert.equal(chart.group.points[0].bytes, 1572864);
  assert.equal(chart.group.points[0].repetitions, undefined);
  assert.equal(chart.description.kind, 'size');
  assert.equal(chart.frameworks[0].color, '#34d399');
  assert.equal(chart.group.detail, 'Linux · x86_64');
  assert.equal(chart.group.label, 'Production executable');
  assert.equal(binarySizeChart({ ...snapshot(), profile: 'release' }).group.label, 'Release executable');
});

test('binary sizes reject invalid measurements and duplicate frameworks', () => {
  for (const bytes of [0, -1, 1.5, Infinity, '1048576']) {
    const source = snapshot();
    source.results[0].bytes = bytes;
    assert.throws(() => binarySizeChart(source));
  }
  const duplicate = snapshot();
  duplicate.results.push({ ...duplicate.results[0] });
  assert.throws(() => binarySizeChart(duplicate));
  assert.throws(() => binarySizeChart({ ...snapshot(), results: [] }));
  assert.throws(() => binarySizeChart({ ...snapshot(), collectedAt: 'invalid' }));
  assert.throws(() => binarySizeChart({ ...snapshot(), profile: 'dev' }));
});
