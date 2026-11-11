import { compactChart } from './compact-chart.js';
import { workloadSections } from '../lib/workloads.mjs';

const element = (tag, text, className) => {
  const result = document.createElement(tag);
  if (text !== undefined) result.textContent = text;
  if (className) result.className = className;
  return result;
};
const number = value => value.toLocaleString(undefined, { maximumFractionDigits: 4 });
const runSelector = document.querySelector('#run');
const status = document.querySelector('#chart-status');
const defaultHidden = ['freya', 'vizia', 'whisker', 'blinc'];
let hidden = new Set(defaultHidden);
const charts = [];
const resizeObserver = new ResizeObserver(() => charts.forEach(chart => chart.resize()));
const chartContainer = document.querySelector('#charts');
resizeObserver.observe(chartContainer);

function dataTable(points, missing, frameworks, versions, description) {
  const isMemory = description.kind === 'memory';
  const details = element('details', undefined, 'chart-data');
  details.append(element('summary', `Values and unavailable results (${points.length + missing.length})`));
  const scroll = element('div', undefined, 'table-scroll');
  const table = element('table');
  const head = element('thead');
  const headings = element('tr');
  for (const title of ['Framework', 'Version', isMemory ? 'Average' : 'Median', ...(isMemory ? [] : ['Range']), 'Repetitions / status']) headings.append(element('th', title));
  head.append(headings);
  const body = element('tbody');
  for (const point of [...points, ...missing]) {
    const row = element('tr');
    const framework = element('th', frameworks.get(point.framework)?.name ?? point.framework);
    framework.scope = 'row';
    row.append(framework, element('td', versions[point.framework] ?? 'Unreported'));
    if (point.status === 'ok') row.append(element('td', `${number(point.value)} ${point.unit}`), ...(isMemory ? [] : [element('td', `${number(point.minimum)}–${number(point.maximum)}`)]), element('td', String(point.repetitions)));
    else row.append(element('td', 'Unavailable'), ...(isMemory ? [] : [element('td', 'Not plotted')]), element('td', `${point.status}${point.reason ? `: ${point.reason}` : ''}`));
    body.append(row);
  }
  table.append(head, body);
  scroll.append(table);
  details.append(scroll);
  return details;
}

async function start() {
  const response = await fetch('/data/benchmarks.json');
  if (!response.ok) throw new Error(`Loading chart data: HTTP ${response.status}`);
  const dashboard = await response.json();
  if (!dashboard.runs.length) {
    status.textContent = 'No measurements yet. The feature comparison is available below.';
    return;
  }
  let selected = dashboard.runs[0];

  function renderCharts() {
    charts.splice(0).forEach(chart => chart.dispose());
    chartContainer.replaceChildren();
    const workloads = workloadSections(selected);
    status.hidden = workloads.some(workload => workload.groups.length);
    status.textContent = 'This collection has no compatible measurements for these workloads.';
    const frameworks = new Map(selected.frameworks.map(framework => [framework.id, framework]));
    let previousScenario;
    let workloadHeading;
    for (const workload of workloads) {
      const { metric } = workload;
      const description = dashboard.metrics[metric];
      const section = element('section', undefined, 'workload-section');
      section.dataset.scenario = workload.scenario;
      section.dataset.metric = metric;
      if (workload.scenario !== previousScenario) {
        workloadHeading = element('h3', workload.label);
        workloadHeading.id = `workload-${workload.scenario}-${metric}`;
        section.append(workloadHeading);
      } else {
        section.classList.add('workload-continuation');
        chartContainer.lastElementChild?.classList.add('workload-has-continuation');
      }
      previousScenario = workload.scenario;
      section.setAttribute('aria-labelledby', workloadHeading.id);
      section.append(element('p', `${description.label} (${description.unit})`, 'measurement-label'));
      if (description.note) section.append(element('p', description.note, 'muted'));
      chartContainer.append(section);
      if (!workload.groups.length) section.append(element('p', 'This collection has no compatible measurements for this workload.', 'notice'));
      for (const group of workload.groups) {
        const points = group.points.filter(point => !hidden.has(point.framework));
        const missing = group.missing.filter(point => !hidden.has(point.framework));
        const panel = element('div', undefined, 'chart-panel');
        panel.append(element('h4', `${group.label} · workload v${group.scenarioVersion}${group.boundary ? ` · ${group.boundary}` : ''}`, 'sr-only'));
        section.append(panel);
        if (points.length) {
          const host = element('div', undefined, 'echart');
          host.setAttribute('role', 'img');
          host.setAttribute('aria-label', `${description.label} for ${group.label}, version ${group.scenarioVersion}. Exact values are in the following expandable table.`);
          const frame = element('div', undefined, 'compact-chart-frame');
          frame.tabIndex = 0;
          frame.setAttribute('role', 'region');
          frame.setAttribute('aria-label', 'Benchmark plot, scroll horizontally on small screens');
          frame.append(host);
          panel.append(frame);
          charts.push(compactChart(host, { points, missing, frameworks, group, metric, description }));
        } else {
          panel.append(element('p', group.points.length ? 'No measured frameworks are visible. Use “Show all” to restore them.' : 'No successful measurements in this collection. See unavailable results below.', 'notice'));
          for (const point of missing) panel.append(element('p', `${frameworks.get(point.framework)?.name ?? point.framework}: no data`, 'muted'));
        }
        panel.append(dataTable(points, missing, frameworks, group.versions, description));
      }
    }
    for (const button of document.querySelectorAll('#toggles button')) button.setAttribute('aria-pressed', String(!hidden.has(button.dataset.framework)));
  }

  function renderCollection() {
    document.querySelector('#toggles').replaceChildren(...selected.frameworks.map(framework => {
      const button = element('button', framework.name, 'framework-toggle');
      button.type = 'button';
      button.dataset.framework = framework.id;
      button.style.setProperty('--framework-color', framework.color);
      button.addEventListener('click', () => {
        if (hidden.has(framework.id)) hidden.delete(framework.id);
        else hidden.add(framework.id);
        renderCharts();
      });
      return button;
    }));
    renderCharts();
  }

  runSelector.value = selected.id;
  runSelector.addEventListener('change', () => {
    selected = dashboard.runs.find(run => run.id === runSelector.value) ?? dashboard.runs[0];
    hidden = new Set(defaultHidden);
    renderCollection();
  });
  document.querySelector('#reset').addEventListener('click', () => { hidden.clear(); renderCharts(); });
  renderCollection();
  for (const control of [runSelector, document.querySelector('#reset')]) control.disabled = false;
}

start().catch(error => {
  status.hidden = false;
  status.textContent = `Could not render charts: ${error.message}. The feature comparison remains available below.`;
});
