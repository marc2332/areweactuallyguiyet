export const metricLabels = {
  renderer_fps: { label: 'Present calls per second', unit: 'frames/s' },
  renderer_frames: { label: 'Present calls', unit: 'frames' },
  frame_gap_p95_ms: { label: 'p95 present-call gap', unit: 'ms' },
  frame_gap_p99_ms: { label: 'p99 present-call gap', unit: 'ms' },
  frame_gap_max_ms: { label: 'Longest present-call gap', unit: 'ms' },
  cpu_ms: { label: 'CPU time', unit: 'ms' },
  rss_mib: { label: 'RSS', unit: 'MiB' },
  pss_mib: { label: 'PSS', unit: 'MiB' },
  peak_rss_mib: { label: 'Peak RSS', unit: 'MiB' },
};

export const frameworkColors = {
  freya: '#34d399',
  'freya-optimized': '#ef4444',
  'freya-attemp-1': '#2dd4bf',
  'freya-attempt-2': '#a7f3d0',
  'freya-attempt-3': '#6ee7b7',
  'freya-attempt-4': '#86efac',
  'freya-attempt-5': '#4ade80',
  'freya-attempt-6': '#bbf7d0',
  'freya-attempt-7': '#22c55e',
  'freya-attempt-8': '#d9f99d',
  egui: '#60a5fa',
  vizia: '#f472b6',
  'vizia-optimized': '#e879f9',
  iced: '#22d3ee',
  gpui: '#fb923c',
  'gpui-fast': '#c084fc',
  'dioxus-native': '#facc15',
  'dioxus-native-vello': '#14b8a6',
  'dioxus-native-vello-hybrid': '#d946ef',
  slint: '#e2e8f0',
  floem: '#a3e635',
  makepad: '#818cf8',
  rune: '#fda4af',
  'vizia-rayon': '#fb7185',
  'dioxus-native-skia': '#fbbf24',
};

export const scenarioLabels = {
  scroll: 'Scroll 2,000 rows',
};

export function frameworkColor(identifier) {
  if (frameworkColors[identifier]) return frameworkColors[identifier];
  const palette = ['#facc15', '#a3e635', '#e2e8f0', '#fb7185'];
  const hash = [...identifier].reduce((value, character) => (value * 31 + character.charCodeAt(0)) >>> 0, 0);
  return palette[hash % palette.length];
}

export function metadataForScenario(run, scenario) {
  return run.metadata.scenarioCollections?.[scenario] ?? run.metadata;
}

export function frameworkVersionsForScenario(run, scenario) {
  return metadataForScenario(run, scenario).frameworkVersions ?? {};
}

export function responsivenessDetails(run) {
  const scenarios = Object.entries(run.metadata.scenarioCollections ?? {})
    .filter(([, metadata]) => metadata.responsivenessVersion)
    .map(([scenario]) => scenarioLabels[scenario] ?? scenario);
  if (scenarios.length) return `${scenarios.join(', ')}: renderer cadence measured, other workloads inherited. Visible updates unverified`;
  return run.metadata.responsivenessVersion
    ? 'Renderer cadence measured, visible updates unverified'
    : 'Responsiveness unmeasured, resource costs are not speed rankings';
}

export function rendererBoundary(result) {
  const boundaries = [...new Set(result.repetitions.flatMap(repetition =>
    (repetition.events ?? []).flatMap(event => ['frame-activity', 'mangohud-capture'].includes(event.event) ? [event.trace.boundary] : [])
  ))].filter(Boolean);
  return boundaries.length === 1 ? boundaries[0] : 'unreported-or-inconsistent';
}

export function groupMetricResults(run, metric, hidden = new Set()) {
  const scenarios = [...new Set([...Object.keys(scenarioLabels), ...run.results.map(result => result.scenario)])];
  const groups = [];
  for (const scenario of scenarios) {
    const results = run.results.filter(result => result.scenario === scenario);
    const versions = [...new Set(results.map(result => result.scenarioVersion))];
    for (const scenarioVersion of versions) {
      const versioned = results.filter(result => result.scenarioVersion === scenarioVersion);
      const rendererMetric = metric.startsWith('renderer_') || metric.startsWith('frame_gap_');
      const boundaries = rendererMetric
        ? [...new Set(versioned.map(rendererBoundary))].filter(boundary => ['present-returned', 'mangohud-present-observation'].includes(boundary))
        : [null];
      for (const boundary of boundaries) {
        const compatible = boundary === null ? versioned : versioned.map(result =>
          result.status === 'ok' && rendererBoundary(result) !== boundary
            ? { ...result, status: 'unsupported', reason: `No compatible ${boundary} capture. Other renderer boundaries are excluded.` }
            : result
        );
        if (!compatible.some(result => result.status === 'ok' && result.metrics[metric])) continue;
        const transformed = transformChartResults(compatible, { metric, scenarioVersion, hidden });
        const points = transformed.filter(point => point.status === 'ok').map(point => {
          const source = compatible.find(result => result.framework === point.framework);
          const repetitions = source.repetitions.flatMap(repetition => {
            const sample = repetition.metrics?.[metric];
            return sample && sample.unit === point.unit && Number.isFinite(sample.value) && sample.value >= 0 ? [sample.value] : [];
          });
          return { ...point, minimum: Math.min(point.value, ...repetitions), maximum: Math.max(point.value, ...repetitions), repetitions: repetitions.length };
        });
        groups.push({ scenario, scenarioVersion, boundary, label: scenarioLabels[scenario] ?? scenario, points, missing: transformed.filter(point => point.status !== 'ok') });
      }
    }
  }
  return groups;
}

export function placePointLabels(points, { left, right, width, axisMaximum, names }) {
  const positioned = points.map(point => {
    const horizontal = left + point.value / axisMaximum * (right - left);
    const lower = left + point.minimum / axisMaximum * (right - left);
    const upper = left + point.maximum / axisMaximum * (right - left);
    const label = names[point.framework] ?? point.framework;
    const labelWidth = label.length * 6 + 54;
    const labelOnLeft = horizontal + 9 + labelWidth > width - 18;
    return { ...point, horizontal, lower, upper, labelOnLeft,
      start: Math.min(lower - 5, labelOnLeft ? horizontal - 9 - labelWidth : horizontal - 5),
      end: Math.max(upper + 5, labelOnLeft ? horizontal + 5 : horizontal + 9 + labelWidth) };
  }).sort((first, second) => first.start - second.start);
  const laneEnds = [];
  for (const point of positioned) {
    let lane = laneEnds.findIndex(end => end + 8 <= point.start);
    if (lane === -1) lane = laneEnds.length;
    laneEnds[lane] = point.end;
    point.lane = lane;
  }
  return { points: positioned, laneCount: laneEnds.length };
}

export function filterRunsByMachine(runs, machineId) {
  return runs.filter(run => !machineId || run.machine.id === machineId);
}

export function transformChartResults(results, { metric, scenarioVersion, relative = false, hidden = new Set() }) {
  const visible = results.filter(result => !hidden.has(result.framework));
  const candidates = visible.filter(result => result.status === 'ok' && result.scenarioVersion === scenarioVersion && result.metrics[metric]);
  const units = new Set(candidates.map(result => result.metrics[metric].unit));
  const compatible = units.size === 1 ? candidates : [];
  const positive = compatible.map(result => result.metrics[metric].value).filter(value => value > 0);
  const fastest = positive.length ? Math.min(...positive) : null;
  return visible.map(result => {
    if (result.scenarioVersion !== scenarioVersion) return { framework: result.framework, status: 'incompatible-scenario-version', value: null, unit: null };
    if (result.status !== 'ok') return { framework: result.framework, status: result.status, value: null, unit: null, reason: result.reason };
    const measurement = result.metrics[metric];
    if (!measurement) return { framework: result.framework, status: 'missing-metric', value: null, unit: null };
    if (units.size !== 1) return { framework: result.framework, status: 'incompatible-unit', value: null, unit: measurement.unit };
    if (relative && measurement.value === 0) return { framework: result.framework, status: 'zero-relative-unavailable', value: null, unit: '×' };
    if (relative && fastest === null) return { framework: result.framework, status: 'relative-unavailable', value: null, unit: '×' };
    return { framework: result.framework, status: 'ok', value: relative ? measurement.value / fastest : measurement.value, unit: relative ? '×' : measurement.unit };
  });
}

export function collectionDetails(run) {
  const timing = `${run.metadata.durationSeconds ?? '?'}s runs × ${run.metadata.repetitions ?? '?'} · ${run.metadata.warmupSeconds ?? '?'}s warm-up`;
  const scenarioTiming = Object.entries(run.metadata.scenarioCollections ?? {})
    .map(([scenario, metadata]) => {
      const refreshRates = (metadata.displayRefreshProbe?.activeModes ?? []).flatMap(line =>
        line.trim().split(/\s+/).filter(field => field.includes('*')).map(field => field.replace(/[*+]/g, ''))
      ).filter(rate => Number.isFinite(Number(rate)) && Number(rate) > 0);
      const refresh = refreshRates.length ? ` · monitor mode ${refreshRates.join('/')}Hz` : '';
      return ` · ${scenarioLabels[scenario] ?? scenario}: ${metadata.durationSeconds ?? '?'}s × ${metadata.repetitions ?? '?'} · ${metadata.warmupSeconds ?? '?'}s warm-up${refresh}`;
    })
    .join('');
  return { timing: `${timing}${scenarioTiming}` };
}
