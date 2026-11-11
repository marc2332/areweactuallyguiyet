export function scrollSeries(groups, hidden = new Set()) {
  const rows = [...new Set(groups.map(group => group.rows))].sort((first, second) => first - second);
  const series = new Map();
  for (const group of groups) {
    for (const point of group.points) {
      if (hidden.has(point.framework)) continue;
      const key = `${point.framework}/${group.scenarioVersion}/${group.boundary}`;
      if (!series.has(key)) series.set(key, {
        framework: point.framework, scenarioVersion: group.scenarioVersion, boundary: group.boundary,
        samples: rows.map(rows => ({ rows, point: null })),
      });
      series.get(key).samples.find(sample => sample.rows === group.rows).point = point;
    }
  }
  return { rows, series: [...series.values()] };
}
