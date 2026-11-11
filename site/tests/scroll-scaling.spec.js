import { expect, test } from '@playwright/test';

test('latest measurements show every library across all four row counts', async ({ page }, testInfo) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  const dashboard = await (await page.request.get('/data/benchmarks.json')).json();
  const latest = dashboard.runs[0];
  expect(latest.measurement).toBe('mangohud-scroll-v3');
  const groups = latest.groups.renderer_fps;
  expect(groups.map(group => group.rows)).toEqual([1000, 3000, 5000, 7000]);
  for (const group of groups) {
    expect(group.points).toHaveLength(17);
    expect(group.points.every(point => point.repetitions === 3)).toBe(true);
    expect(group.missing).toEqual([]);
  }
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('#run')).toBeEnabled();
  await page.getByRole('button', { name: 'Show all', exact: true }).click();
  await expect(page.locator('#toggles button[aria-pressed="true"]')).toHaveCount(17);
  const scroll = page.locator('[data-scenario="scroll"]');
  await expect(scroll.locator('.echart svg')).toHaveCount(2);
  await expect(scroll.locator('h3')).toHaveText('Scroll scaling');
  const cadence = page.locator('[data-metric="renderer_fps"]');
  await expect(cadence.locator('.chart-data')).toHaveCount(1);
  await expect(cadence.locator('.chart-data summary')).toHaveText('Values and unavailable results (68)');
  await expect(cadence.locator('.chart-data table').first()).toBeHidden();
  await cadence.locator('.chart-data summary').click();
  await expect(cadence.locator('.chart-data table')).toHaveCount(4);
  await expect(cadence.locator('.chart-data tbody tr')).toHaveCount(68);
  await expect(page.locator('[data-scenario="dashboard"] .echart svg')).toHaveCount(2);
  await cadence.locator('.echart').screenshot({ path: testInfo.outputPath('latest-scroll.png') });
  expect(errors).toEqual([]);
});

test('scroll scaling renders four sizes, ranges, filters and exact values', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.route('**/data/benchmarks.json', async route => {
    const response = await route.fetch();
    const dashboard = await response.json();
    const run = dashboard.runs[0];
    run.frameworks = [
      { id: 'egui', name: 'egui', color: '#60a5fa' },
      { id: 'iced', name: 'Iced', color: '#22d3ee' },
      { id: 'freya', name: 'Freya', color: '#34d399' },
    ];
    run.groups = Object.fromEntries(['renderer_fps', 'frame_gap_p95_ms'].map(metric => [metric,
      [1000, 3000, 5000, 7000].map((rows, index) => ({
        scenario: `scroll-${rows}`, rows, scenarioVersion: 3, boundary: 'mangohud-present-observation',
        label: `Scroll ${rows.toLocaleString('en-US')} rows`, versions: { egui: 'fixture', iced: 'fixture' }, missing: [],
        points: run.frameworks.map((framework, frameworkIndex) => {
          const tooltipValues = metric === 'renderer_fps' ? [30 + 1 / 3, 90 + 2 / 3, 60 + 7 / 9] : [30 + 1 / 3, 10 + 2 / 3, 20 + 7 / 9];
          const value = index === 1 ? tooltipValues[frameworkIndex]
            : metric === 'renderer_fps' ? 144 - index * (frameworkIndex ? 35 : 3) : 7 + index * (frameworkIndex ? 10 : 1);
          return { framework: framework.id, status: 'ok', value, minimum: value - 1, maximum: value + 1,
            unit: metric === 'renderer_fps' ? 'frames/s' : 'ms', repetitions: 3 };
        }),
      })),
    ]));
    await route.fulfill({ response, json: dashboard });
  });
  await page.goto('/');
  await expect(page.locator('#run')).toBeEnabled();
  await page.getByRole('button', { name: 'Show all', exact: true }).click();
  const plots = page.locator('[data-scenario="scroll"] .echart');
  await expect(plots).toHaveCount(2);
  for (const plot of await plots.all()) {
    await expect(plot.locator('svg')).toBeVisible();
    for (const rows of ['1,000', '3,000', '5,000', '7,000']) await expect(plot.locator('svg text').filter({ hasText: new RegExp(`^${rows}$`) })).toHaveCount(1);
    await expect(plot).toContainText('egui');
    await expect(plot).toContainText('Iced');
    await expect(plot).toContainText('Freya');
  }
  for (const [index, labels] of [[0, ['90.67', '60.78', '30.33']], [1, ['10.67', '20.78', '30.33']]]) {
    const plot = plots.nth(index);
    const bounds = await plot.boundingBox();
    await plot.hover({ position: { x: 65 + (bounds.width - 95) / 3, y: 100 } });
    const positions = [];
    for (const label of labels) {
      const value = plot.locator('svg text').filter({ hasText: new RegExp(`^${label.replace('.', '\\.')}$`) });
      await expect(value).toBeVisible();
      positions.push((await value.boundingBox()).y);
    }
    expect(positions[0]).toBeLessThan(positions[1]);
    expect(positions[1]).toBeLessThan(positions[2]);
    await expect(plot).not.toContainText('30.333333');
  }
  const cadence = page.locator('[data-metric="renderer_fps"]');
  const values = cadence.locator('.chart-data');
  await expect(values).toHaveCount(1);
  await expect(values.locator('summary')).toHaveText('Values and unavailable results (12)');
  await expect(values.locator('table').first()).toBeHidden();
  await values.locator('summary').click();
  await expect(values.locator('table')).toHaveCount(4);
  for (const table of await values.locator('table').all()) await expect(table).toBeVisible();
  await expect(values).toContainText('143–145');
  await values.locator('summary').click();
  for (const table of await values.locator('table').all()) await expect(table).toBeHidden();
  await page.locator('#toggles button[data-framework="egui"]').click();
  await expect(plots.first()).not.toContainText('egui');
  await expect(values.locator('summary')).toHaveText('Values and unavailable results (8)');
  await page.locator('#toggles button[data-framework="iced"]').click();
  await page.locator('#toggles button[data-framework="freya"]').click();
  await expect(plots).toHaveCount(0);
  await expect(cadence).toContainText('No measured frameworks are visible');
  await page.getByRole('button', { name: 'Show all', exact: true }).click();
  await expect(plots).toHaveCount(2);
  expect(errors).toEqual([]);
});
