import { init, use } from 'echarts/core';
import { CustomChart, LineChart } from 'echarts/charts';
import { AriaComponent, GridComponent, LegendComponent, TooltipComponent } from 'echarts/components';
import { SVGRenderer } from 'echarts/renderers';
import { scrollSeries } from '../lib/scroll-scaling.mjs';

use([CustomChart, LineChart, AriaComponent, GridComponent, LegendComponent, TooltipComponent, SVGRenderer]);

export function scrollChart(host, { groups, hidden, frameworks, description }) {
  host.style.height = '420px';
  const chart = init(host, null, { renderer: 'svg' });
  const { rows, series } = scrollSeries(groups, hidden);
  chart.setOption({
    animation: !window.matchMedia('(prefers-reduced-motion: reduce)').matches,
    animationDuration: 220,
    backgroundColor: '#111110',
    color: series.map(series => frameworks.get(series.framework)?.color ?? '#cbd5e1'),
    textStyle: { color: '#999993', fontFamily: getComputedStyle(host).fontFamily },
    aria: { enabled: true, description: `${description.label} by row count. Lines show repetition medians, whiskers show repetition ranges. Exact values follow.` },
    grid: { left: 65, right: 30, top: 35, bottom: 110 },
    legend: { type: 'scroll', bottom: 0, textStyle: { color: '#e6e6e2' } },
    tooltip: { trigger: 'axis', renderMode: 'richText', confine: true,
      order: description.unit === 'frames/s' ? 'valueDesc' : 'valueAsc',
      valueFormatter: value => Number(value).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 }) },
    xAxis: { type: 'category', name: 'Rows', data: rows.map(String), boundaryGap: false,
      axisLabel: { color: '#999993', interval: 0, formatter: value => Number(value).toLocaleString() },
      splitLine: { lineStyle: { color: '#33332f' } } },
    yAxis: { type: 'value', name: description.unit, scale: true, boundaryGap: ['10%', '10%'],
      axisLabel: { color: '#999993' }, splitLine: { lineStyle: { color: '#33332f' } } },
    series: series.flatMap(series => {
      const framework = frameworks.get(series.framework);
      const incompatible = groups.some(group => group.scenarioVersion !== series.scenarioVersion || group.boundary !== series.boundary);
      const name = `${framework?.name ?? series.framework}${incompatible ? ` · v${series.scenarioVersion} · ${series.boundary}` : ''}`;
      const color = framework?.color ?? '#cbd5e1';
      return [{
        name, type: 'line', connectNulls: false, symbol: 'circle', symbolSize: 7,
        itemStyle: { color }, lineStyle: { color },
        data: series.samples.map(({ rows, point }) => [String(rows), point?.value ?? null]),
      }, {
        name, type: 'custom', silent: true, tooltip: { show: false },
        data: series.samples.flatMap(({ rows, point }) => point ? [[String(rows), point.minimum, point.maximum]] : []),
        encode: { x: 0, y: [1, 2] },
        renderItem: (parameters, api) => {
          const lower = api.coord([api.value(0), api.value(1)]);
          const upper = api.coord([api.value(0), api.value(2)]);
          return { type: 'group', children: [
            { type: 'line', shape: { x1: lower[0], y1: lower[1], x2: upper[0], y2: upper[1] }, style: { stroke: color, opacity: 0.6 } },
            ...[lower, upper].map(position => ({ type: 'line', shape: { x1: position[0] - 4, y1: position[1], x2: position[0] + 4, y2: position[1] }, style: { stroke: color, opacity: 0.6 } })),
          ] };
        },
      }];
    }),
  });
  return { resize: () => chart.resize(), dispose: () => chart.dispose() };
}
