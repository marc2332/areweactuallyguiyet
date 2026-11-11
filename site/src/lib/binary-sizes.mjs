import { frameworkColor } from './chart-data.mjs';

export function binarySizeChart(snapshot) {
  if (snapshot.schemaVersion !== 1 || !Number.isFinite(Date.parse(snapshot.collectedAt)) || !['release', 'production'].includes(snapshot.profile)
    || !snapshot.platform?.os || !snapshot.platform?.architecture || !snapshot.method || !Array.isArray(snapshot.results) || !snapshot.results.length) {
    throw new Error('Invalid binary size snapshot');
  }
  const identifiers = new Set();
  for (const result of snapshot.results) {
    if (!result.framework || identifiers.has(result.framework) || !result.name || !result.binary || !Number.isSafeInteger(result.bytes) || result.bytes <= 0) {
      throw new Error('Invalid or duplicate binary size result');
    }
    identifiers.add(result.framework);
  }
  return {
    ...snapshot,
    frameworks: snapshot.results.map(result => ({ id: result.framework, name: result.name, color: frameworkColor(result.framework) })),
    description: { label: 'Binary size', unit: 'MiB', kind: 'size' },
    group: {
      scenario: 'binary-size',
      label: snapshot.profile === 'production' ? 'Production executable' : 'Release executable',
      detail: `${snapshot.platform.os} · ${snapshot.platform.architecture}`,
      versions: Object.fromEntries(snapshot.results.map(result => [result.framework, result.binary])),
      points: snapshot.results.map(result => {
        const value = result.bytes / (1024 * 1024);
        return { framework: result.framework, status: 'ok', value, minimum: value, maximum: value, unit: 'MiB', bytes: result.bytes };
      }),
    },
  };
}
