import { init, use } from 'echarts/core';
import { CustomChart } from 'echarts/charts';
import { AriaComponent, DataZoomComponent, GraphicComponent, GridComponent, ToolboxComponent, TooltipComponent } from 'echarts/components';
import { SVGRenderer } from 'echarts/renderers';

use([CustomChart, AriaComponent, DataZoomComponent, GraphicComponent, GridComponent, ToolboxComponent, TooltipComponent, SVGRenderer]);

const number = value => value.toLocaleString(undefined, { maximumFractionDigits: 4 });
const labelNumber = value => value.toLocaleString(undefined, { maximumFractionDigits: value < 10 ? 2 : 1 });

function axisScale(maximum) {
  if (maximum <= 0) return { maximum: 1, interval: 0.2 };
  const magnitude = 10 ** Math.floor(Math.log10(maximum / 6));
  const target = maximum / 6 / magnitude;
  const multiple = [1, 2, 2.5, 5, 10].reduce((best, value) => Math.abs(value - target) < Math.abs(best - target) ? value : best);
  const interval = multiple * magnitude;
  return { maximum: Math.ceil(maximum / interval) * interval, interval };
}

function placeLeftLabels(points, { left, right, minimum, maximum, labelWidths }) {
  const positioned = points.map(point => {
    const horizontal = left + (point.value - minimum) / (maximum - minimum) * (right - left);
    const lower = left + (point.minimum - minimum) / (maximum - minimum) * (right - left);
    const upper = left + (point.maximum - minimum) / (maximum - minimum) * (right - left);
    return { ...point, start: Math.min(lower - 4, horizontal - 9 - labelWidths[point.framework]), end: Math.max(upper + 4, horizontal + 4) };
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

export function compactChart(host, { points, missing = [], frameworks, group, metric, description }) {
  host.style.height = '210px';
  const chart = init(host, null, { renderer: 'svg' });
  const minimum = points.length ? Math.min(...points.map(point => point.minimum)) : 0;
  const maximum = points.length ? Math.max(...points.map(point => point.maximum)) : 1;
  const padding = Math.max((maximum - minimum) * 0.1, Math.abs(maximum) * 0.001, 0.0001);
  const lowerBound = Math.max(0, minimum - padding);
  const upperBound = maximum + padding;
  const { interval } = axisScale(upperBound - lowerBound);
  const scale = {
    minimum: Math.floor(lowerBound / interval) * interval,
    maximum: Math.ceil(upperBound / interval) * interval,
    interval,
  };
  const isSize = description.kind === 'size';
  const isMemory = description.kind === 'memory';
  const detail = group.detail ?? `${group.boundary ? `${group.boundary} · ` : ''}v${group.scenarioVersion}`;
  const names = Object.fromEntries(points.map(point => [point.framework, `${frameworks.get(point.framework)?.name ?? point.framework}${isMemory ? '' : ` ${labelNumber(point.value)}`}`]));
  const fontFamily = getComputedStyle(host).fontFamily;
  const context = document.createElement('canvas').getContext('2d');
  if (context) context.font = `11px ${fontFamily}`;
  const labelWidths = Object.fromEntries(Object.entries(names).map(([identifier, name]) => [identifier, context?.measureText(name).width ?? name.length * 7]));
  let previousWidth = 0;

  function resize() {
    const width = host.clientWidth;
    if (width === previousWidth) return;
    previousWidth = width;
    const workloadWidth = 205;
    const right = width - (isMemory ? 300 : 36);
    const plotWidth = right - workloadWidth;
    const labelGutter = isMemory ? Math.max(0, Math.ceil(Math.max(0, ...Object.values(labelWidths))) - workloadWidth + 36) : Math.ceil(Math.max(0, ...points.map(point => {
      const fraction = (point.value - scale.minimum) / (scale.maximum - scale.minimum);
      if (fraction >= 1) return 0;
      return (labelWidths[point.framework] + 17 - fraction * plotWidth) / (1 - fraction);
    })));
    const left = workloadWidth + labelGutter;
    const top = 62;
    const laneSpacing = 16;
    const layout = isMemory
      ? { points: points.map((point, index) => ({ ...point, lane: index })), laneCount: points.length }
      : placeLeftLabels(points, { left, right, minimum: scale.minimum, maximum: scale.maximum, labelWidths });
    const rowHeight = isMemory ? points.length * 42 : Math.max(110, layout.laneCount * laneSpacing + 26);
    const unavailableHeight = missing.length ? missing.length * 18 + 12 : 0;
    host.style.height = `${rowHeight + top + 38 + unavailableHeight}px`;
    chart.resize();
    chart.setOption({
      animation: !window.matchMedia('(prefers-reduced-motion: reduce)').matches,
      animationDuration: 220,
      backgroundColor: '#111110',
      textStyle: { fontFamily, color: '#999993' },
      aria: { enabled: true, description: `${description.label} for ${group.label}. ${isMemory ? 'Connected dots show minimum, average, and maximum application process RSS.' : isSize ? 'Colored dots are executable file sizes.' : 'Frameworks share a workload row. Colored dots are medians and lines are repetition ranges.'} Exact values are provided below.` },
      grid: { left, right: width - right, top, bottom: 38 + unavailableHeight, show: true, backgroundColor: '#171716', borderColor: '#33332f' },
      graphic: [
        { type: 'rect', silent: true, z: -1, shape: { x: workloadWidth, y: top, width: right - workloadWidth, height: rowHeight }, style: { fill: '#171716' } },
        { type: 'text', left: isMemory ? 18 : workloadWidth - 78, top: 29, style: { text: isMemory ? 'Library' : isSize ? 'Artifact' : 'Workload', fill: '#999993', font: `11px ${fontFamily}` } },
        ...(isMemory ? ['Min', 'Average', 'Max'].map((label, index) => ({
          type: 'text', x: right + 85 + index * 90, y: 36,
          style: { text: `${label} (MiB)`, fill: '#999993', font: `11px ${fontFamily}`, align: 'right' },
        })) : []),
        { type: 'text', left: workloadWidth, top: 13, style: { text: `${description.label} (${description.unit})`, fill: '#999993', font: `11px ${fontFamily}` } },
        ...missing.map((point, index) => ({
          type: 'text', silent: true, left: workloadWidth, top: top + rowHeight + 38 + index * 18,
          style: { text: `${frameworks.get(point.framework)?.name ?? point.framework}: no data`, fill: frameworks.get(point.framework)?.color ?? '#999993', font: `11px ${fontFamily}` },
        })),
      ],
      tooltip: { trigger: 'item', renderMode: 'richText', confine: true, backgroundColor: '#171716', borderColor: '#55554e', textStyle: { color: '#e6e6e2', fontSize: 12, fontFamily } },
      toolbox: { right: 14, top: 2, iconStyle: { borderColor: '#999993' }, feature: { dataZoom: { yAxisIndex: 'none', title: { zoom: 'Select horizontal zoom', back: 'Undo zoom' } }, restore: { title: 'Reset view' }, saveAsImage: { title: 'Save SVG', type: 'svg', name: isSize ? 'binary-sizes' : `${metric}-${group.scenario}-v${group.scenarioVersion}`, backgroundColor: '#111110' } } },
      xAxis: ['top', 'bottom'].map(position => ({
        type: 'value', position, min: scale.minimum, max: scale.maximum, interval: scale.interval,
        axisLabel: { color: '#999993', fontSize: 10, formatter: value => `${labelNumber(value)} ${description.unit}`, showMinLabel: true, showMaxLabel: true },
        axisLine: { show: false }, axisTick: { show: false },
        splitLine: { show: position === 'bottom', lineStyle: { color: '#33332f' } },
      })),
      yAxis: {
        type: 'category', data: isMemory ? layout.points.map(point => point.framework) : [group.label], inverse: isMemory,
        axisTick: { show: false }, axisLine: { show: false },
        splitLine: { show: isMemory, lineStyle: { color: '#33332f', opacity: 0.5 } },
        axisLabel: isMemory ? {
          margin: 18, color: '#e6e6e2', fontSize: 11,
          formatter: identifier => frameworks.get(identifier)?.name ?? identifier,
        } : { margin: labelGutter + 18, formatter: () => `{workload|${group.label}}\n{version|${detail}}`, rich: { workload: { color: '#e6e6e2', fontSize: 11, lineHeight: 20 }, version: { color: '#999993', fontSize: 10, lineHeight: 18 } } },
      },
      series: layout.points.map(point => {
        const framework = frameworks.get(point.framework);
        const color = framework?.color ?? '#cbd5e1';
        return {
          type: 'custom', name: framework?.name ?? point.framework, clip: false,
          data: [[point.value, isMemory ? point.lane : 0, point.minimum, point.maximum]], encode: { x: [0, 2, 3], y: 1 },
          tooltip: { formatter: () => isSize
            ? `${framework?.name ?? point.framework}\n${group.versions[point.framework]}\nSize: ${number(point.value)} MiB\n${point.bytes.toLocaleString()} bytes`
            : `${framework?.name ?? point.framework}\n${group.versions[point.framework] ?? 'Version unreported'}\n${isMemory ? 'Average' : 'Median'}: ${number(point.value)} ${point.unit}\n${isMemory ? 'Minimum' : 'Range'}: ${number(point.minimum)}${isMemory ? ` ${point.unit}\nMaximum: ` : '–'}${number(point.maximum)} ${point.unit}\nRepetitions: ${point.repetitions}` },
          renderItem: (parameters, api) => {
            const row = isMemory ? point.lane : 0;
            const center = api.coord([point.value, row]);
            const lower = api.coord([point.minimum, row]);
            const upper = api.coord([point.maximum, row]);
            if (center[0] < left || center[0] > right) return null;
            lower[0] = Math.max(left, Math.min(right, lower[0]));
            upper[0] = Math.max(left, Math.min(right, upper[0]));
            const vertical = isMemory ? center[1] : top + 15 + point.lane * laneSpacing;
            return {
              type: 'group',
              children: [
                { type: 'line', shape: { x1: lower[0], y1: vertical, x2: upper[0], y2: vertical }, style: { stroke: color, lineWidth: 2, opacity: 0.65 } },
                ...(isSize ? [] : [lower, upper]).map(position => isMemory
                  ? { type: 'circle', shape: { cx: position[0], cy: vertical, r: 3.5 }, style: { fill: '#171716', stroke: color, lineWidth: 1.5 } }
                  : { type: 'line', shape: { x1: position[0], y1: vertical - 3, x2: position[0], y2: vertical + 3 }, style: { stroke: color, lineWidth: 1 } }),
                { type: 'circle', shape: { cx: center[0], cy: vertical, r: 2.8 }, style: { fill: color }, emphasis: { style: { shadowBlur: 8, shadowColor: color } } },
                ...(isMemory
                  ? [point.minimum, point.value, point.maximum].map((value, index) => ({
                    type: 'text', x: right + 85 + index * 90, y: vertical,
                    style: { text: labelNumber(value), fill: color, font: `11px ${fontFamily}`, align: 'right', verticalAlign: 'middle' },
                  }))
                  : [{ type: 'text', x: center[0] - 9, y: vertical, style: { text: names[point.framework], fill: color, font: `11px ${fontFamily}`, align: 'right', verticalAlign: 'middle' } }]),
              ],
            };
          },
        };
      }),
    }, { notMerge: true });
  }

  resize();
  return { resize, dispose: () => chart.dispose() };
}
