import { collectionDetails, frameworkColor, frameworkVersionsForScenario, groupMetricResults, metricLabels, placePointLabels } from './chart-data.mjs';

const svgNamespace = 'http://www.w3.org/2000/svg';
const primaryMetrics = ['cpu_ms', 'pss_mib', 'startup_ms'];
const extraMetrics = ['rss_mib'];
const descriptions = {
  cpu_ms: ['CPU time', 'CPU used during the workload, not frame smoothness.'],
  rss_mib: ['Resident memory', 'RSS at workload end. Shared pages may be counted more than once.'],
  startup_ms: ['Time to initial UI', "Launch to the adapter's ready signal, not display presentation."],
  pss_mib: ['Proportional memory', 'Process-tree PSS, which apportions shared pages between processes.'],
};

function element(tag, text, className) {
  const item = document.createElement(tag);
  if (text !== undefined) item.textContent = text;
  if (className) item.className = className;
  return item;
}

function svgElement(tag, attributes = {}, text) {
  const item = document.createElementNS(svgNamespace, tag);
  for (const [name, value] of Object.entries(attributes)) item.setAttribute(name, value);
  if (text !== undefined) item.textContent = text;
  return item;
}

function number(value) {
  return value.toLocaleString(undefined, { maximumFractionDigits: value < 10 ? 2 : 1 });
}

function axis(maximum) {
  if (maximum <= 0) return { maximum: 1, ticks: [0, 0.25, 0.5, 0.75, 1] };
  const magnitude = 10 ** Math.floor(Math.log10(maximum / 5));
  const target = maximum / 5 / magnitude;
  const multiple = [1, 2, 2.5, 5, 10].reduce((best, value) => Math.abs(Math.log(value / target)) < Math.abs(Math.log(best / target)) ? value : best);
  const step = multiple * magnitude;
  const count = Math.ceil(maximum / step);
  return { maximum: count * step, ticks: Array.from({ length: count + 1 }, (_, index) => index * step) };
}

function plot(groups, metric, names, run) {
  const width = 1600;
  const left = 235;
  const right = width - 24;
  const top = 38;
  const laneSpacing = 13;
  const maximum = Math.max(0, ...groups.flatMap(group => group.points.map(point => point.maximum)));
  const scale = axis(maximum);
  let bottom = top;
  const rows = groups.map(group => {
    const layout = placePointLabels(group.points, { left, right, width, axisMaximum: scale.maximum, names });
    const height = Math.max(52, layout.laneCount * laneSpacing + 12 + (group.missing.length ? 18 : 0));
    const row = { ...group, ...layout, top: bottom, height };
    bottom += height;
    return row;
  });
  const height = bottom + 30;
  const titleId = `plot-${metric}-title`;
  const descriptionId = `plot-${metric}-description`;
  const svg = svgElement('svg', { viewBox: `0 0 ${width} ${height}`, role: 'img', 'aria-labelledby': `${titleId} ${descriptionId}` });
  svg.append(
    svgElement('title', { id: titleId }, descriptions[metric][0]),
    svgElement('desc', { id: descriptionId }, 'Dots show medians, whiskers show repetition ranges. Lower is better.'),
    svgElement('text', { x: left - 18, y: 17, 'text-anchor': 'end', class: 'plot-axis-title' }, 'Workload'),
    svgElement('text', { x: left, y: 17, class: 'plot-axis-title' }, `${descriptions[metric][0]} (${metricLabels[metric].unit}), lower to the left`),
  );
  rows.forEach((row, index) => {
    svg.append(svgElement('rect', { x: 0, y: row.top, width, height: row.height, fill: index % 2 ? '#181818' : '#1d1d1d' }));
    svg.append(svgElement('line', { x1: 0, x2: width, y1: row.top + row.height, y2: row.top + row.height, class: 'plot-grid' }));
  });
  for (const tick of scale.ticks) {
    const horizontal = left + tick / scale.maximum * (right - left);
    svg.append(
      svgElement('line', { x1: horizontal, x2: horizontal, y1: top, y2: bottom, class: 'plot-grid' }),
      svgElement('text', { x: horizontal, y: top - 10, 'text-anchor': tick === 0 ? 'start' : tick === scale.maximum ? 'end' : 'middle', class: 'plot-tick' }, `${number(tick)} ${metricLabels[metric].unit}`),
      svgElement('text', { x: horizontal, y: bottom + 23, 'text-anchor': tick === 0 ? 'start' : tick === scale.maximum ? 'end' : 'middle', class: 'plot-tick' }, `${number(tick)} ${metricLabels[metric].unit}`),
    );
  }
  for (const row of rows) {
    const versions = frameworkVersionsForScenario(run, row.scenario);
    const center = row.top + row.height / 2 - (row.missing.length ? 8 : 0);
    svg.append(
      svgElement('text', { x: left - 18, y: center - 1, 'text-anchor': 'end', class: 'plot-workload' }, row.label),
      svgElement('text', { x: left - 18, y: center + 14, 'text-anchor': 'end', class: 'plot-tick' }, `v${row.scenarioVersion} · ${row.points.length} frameworks`),
    );
    if (!row.points.length) svg.append(svgElement('text', { x: left + 16, y: center, class: 'plot-tick' }, 'No visible measurements'));
    for (const point of row.points) {
      const vertical = center + (point.lane - (row.laneCount - 1) / 2) * laneSpacing;
      const color = frameworkColor(point.framework);
      const version = versions[point.framework] ? ` ${versions[point.framework]}` : '';
      const series = svgElement('g', { class: 'benchmark-series', 'data-framework': point.framework });
      series.append(svgElement('title', {}, `${names[point.framework] ?? point.framework}${version}: ${number(point.value)} ${point.unit}. Repetition range ${number(point.minimum)} to ${number(point.maximum)} ${point.unit}.`));
      if (point.maximum > point.minimum) {
        series.append(
          svgElement('line', { x1: point.lower, x2: point.upper, y1: vertical, y2: vertical, stroke: color, 'stroke-width': 1.5 }),
          svgElement('line', { x1: point.lower, x2: point.lower, y1: vertical - 3, y2: vertical + 3, stroke: color, 'stroke-width': 1.5 }),
          svgElement('line', { x1: point.upper, x2: point.upper, y1: vertical - 3, y2: vertical + 3, stroke: color, 'stroke-width': 1.5 }),
        );
      }
      series.append(
        svgElement('circle', { cx: point.horizontal, cy: vertical, r: 3, fill: color, stroke: '#181818', 'stroke-width': 1 }),
        svgElement('text', { x: point.horizontal + (point.labelOnLeft ? -9 : 9), y: vertical + 3.5, 'text-anchor': point.labelOnLeft ? 'end' : 'start', fill: color, class: 'plot-label' }, `${names[point.framework] ?? point.framework} ${number(point.value)}`),
      );
      svg.append(series);
    }
    if (row.missing.length) {
      const missing = row.missing.map(point => `${names[point.framework] ?? point.framework}: ${point.status === 'missing-metric' ? 'not measured' : point.status}`).join(' · ');
      const note = svgElement('text', { x: left + 12, y: row.top + row.height - 10, class: 'plot-missing' }, missing);
      note.append(svgElement('title', {}, row.missing.map(point => `${names[point.framework] ?? point.framework}: ${point.reason ?? point.status}`).join('\n')));
      svg.append(note);
    }
  }
  return svg;
}

function metricCard(run, metric, hidden, names) {
  const figure = element('figure', undefined, 'benchmark-card');
  const heading = element('div', undefined, 'metric-heading');
  heading.append(element('h3', descriptions[metric][0]), element('span', metricLabels[metric].unit, 'metric-unit'));
  const groups = groupMetricResults(run, metric, hidden);
  figure.append(heading, element('p', descriptions[metric][1], 'metric-description'));
  const chart = element('div', undefined, 'benchmark-chart');
  chart.tabIndex = 0;
  chart.setAttribute('role', 'region');
  chart.setAttribute('aria-label', `${descriptions[metric][0]} comparison, scroll horizontally on small screens`);
  if (groups.length) chart.append(plot(groups, metric, names, run));
  else chart.append(element('p', 'This collection has no measurements for this metric.', 'chart-empty'));
  figure.append(chart);
  return figure;
}

export function newestRuns(history) {
  return [...history.runs].sort((first, second) => Date.parse(second.collectedAt) - Date.parse(first.collectedAt) || second.id.localeCompare(first.id));
}

export function renderBenchmarks(history, libraries) {
  const runs = newestRuns(history);
  const empty = document.querySelector('#empty');
  if (!runs.length) {
    empty.textContent = 'No measurements yet.';
    return;
  }
  const bench = document.querySelector('#bench');
  bench.hidden = false;
  empty.hidden = true;
  const baseNames = Object.fromEntries(libraries.map(library => [library.id, library.name]));
  let names = baseNames;
  const hidden = new Set();
  const selector = document.querySelector('#run');
  document.querySelector('#collection-control').hidden = false;
  selector.replaceChildren(...runs.map((run, index) => {
    const measured = run.results.filter(result => result.status === 'ok').length;
    const option = element('option', `${index === 0 ? 'Latest · ' : ''}${new Date(run.collectedAt).toLocaleString()} · ${run.machine.label} · ${run.metadata.buildProfile ?? 'unknown profile'} · ${measured}/${run.results.length} cases · ${run.id}`);
    option.value = run.id;
    return option;
  }));
  let selected = runs[0];
  selector.value = selected.id;
  let highlighted = null;

  function highlight(identifier) {
    if (hidden.has(identifier)) identifier = null;
    highlighted = identifier;
    bench.classList.toggle('has-highlight', Boolean(identifier));
    for (const series of document.querySelectorAll('#bench .benchmark-series')) series.classList.toggle('is-highlighted', series.dataset.framework === identifier);
  }

  function renderCharts() {
    document.querySelector('#primary-charts').replaceChildren(...primaryMetrics.map(metric => metricCard(selected, metric, hidden, names)));
    document.querySelector('#extra-charts').replaceChildren(...extraMetrics.map(metric => metricCard(selected, metric, hidden, names)));
    for (const button of document.querySelectorAll('#toggles button')) button.setAttribute('aria-pressed', String(!hidden.has(button.dataset.framework)));
    highlight(highlighted);
  }

  function renderCollection() {
    names = { ...baseNames, ...selected.metadata.frameworkLabels };
    highlight(null);
    const details = collectionDetails(selected);
    document.querySelector('#collection-run').textContent = `Run ${selected.id} · ${new Date(selected.collectedAt).toLocaleString()} · ${selected.id === runs[0].id ? 'Latest run' : 'Historical run'} · Showing only this full collection. ${details.provenance}`;
    const identifiers = [...new Set(selected.results.map(result => result.framework))];
    const measured = selected.results.filter(result => result.status === 'ok').length;
    const kind = selected.metadata.hoverAugmentation ? 'augmented collection' : 'collection';
    document.querySelector('#collection-summary').textContent = `${selected.metadata.buildProfile ?? 'Unknown'} ${kind} · ${identifiers.length} frameworks · ${measured} measured cases`;
    document.querySelector('#collection-timing').textContent = details.timing;
    const gpu = (selected.machine.gpu ?? 'GPU unknown').replace(
      '2d:00.0 VGA compatible controller: Advanced Micro Devices, Inc. [AMD/ATI] Device 7590 (rev c0)',
      'AMD Radeon RX 9060 (16 GB VRAM)',
    );
    document.querySelector('#collection-machine').textContent = [
      selected.machine.cpu ?? 'CPU unknown',
      gpu,
      selected.machine.distribution,
      selected.machine.os ?? 'OS unknown',
      selected.machine.session === 'wayland' ? 'Wayland' : selected.machine.session,
    ].filter(Boolean).join(' · ');
    const legacyStartup = selected.results.some(result => result.scenario === 'startup' && result.scenarioVersion < 2);
    document.querySelector('#startup-method-note').textContent = legacyStartup
      ? 'Startup v1 used inconsistent ready signals and memory sampling. Do not compare its startup results.'
      : `Startup memory: sampled after ${selected.metadata.startupMemorySettleSeconds ?? selected.metadata.warmupSeconds ?? '?'}s idle. Settling is excluded from startup timing.`;
    document.querySelector('#toggles').replaceChildren(...identifiers.map(identifier => {
      const button = element('button', undefined, 'framework-toggle');
      button.type = 'button';
      button.dataset.framework = identifier;
      button.style.setProperty('--framework-color', frameworkColor(identifier));
      button.setAttribute('aria-pressed', String(!hidden.has(identifier)));
      const swatch = element('span', undefined, 'chart-swatch');
      swatch.style.color = frameworkColor(identifier);
      swatch.setAttribute('aria-hidden', 'true');
      button.append(swatch, element('span', names[identifier] ?? identifier));
      button.addEventListener('click', () => {
        if (hidden.has(identifier)) hidden.delete(identifier);
        else hidden.add(identifier);
        renderCharts();
      });
      button.addEventListener('mouseenter', () => highlight(identifier));
      button.addEventListener('mouseleave', () => highlight(null));
      button.addEventListener('focus', () => highlight(identifier));
      button.addEventListener('blur', () => highlight(null));
      return button;
    }));
    renderCharts();
  }

  selector.addEventListener('change', () => {
    selected = runs.find(run => run.id === selector.value) ?? runs[0];
    hidden.clear();
    renderCollection();
  });
  renderCollection();
}
