import { expect, test } from '@playwright/test';

async function selectScrollCollection(page) {
  const dashboard = await (await page.request.get('/data/benchmarks.json')).json();
  const run = dashboard.runs.find(collection => collection.groups.renderer_fps?.some(group => ['present-returned', 'mangohud-present-observation'].includes(group.boundary)));
  expect(run).toBeTruthy();
  await page.selectOption('#run', run.id);
}

test('scroll charts support automatic measurements, framework filters and collections', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.goto('/');
  await expect(page.locator('#run')).toBeEnabled();
  await selectScrollCollection(page);
  await expect(page.locator('#scenario, #metric')).toHaveCount(0);
  const workloads = page.locator('.workload-section');
  await expect(workloads).toHaveCount(1);
  await expect(workloads.locator('h3')).toHaveText(['Scroll 2,000 rows']);
  for (const section of await workloads.all()) await expect(section.locator('.echart svg').first()).toBeVisible();
  const scrollWorkload = page.locator('[data-scenario="scroll"]');
  await expect(scrollWorkload).toHaveAttribute('data-metric', 'renderer_fps');

  await expect(page.locator('[data-scenario="startup"]')).toHaveCount(0);
  const dashboard = await (await page.request.get('/data/benchmarks.json')).json();
  for (const run of dashboard.runs) {
    expect(Object.keys(run.groups).some(metric => metric.startsWith('native_'))).toBe(false);
    for (const groups of Object.values(run.groups)) {
      for (const group of groups) expect(group.scenario).toBe('scroll');
    }
  }
  await scrollWorkload.locator('.chart-data > summary').first().click();
  const rows = scrollWorkload.locator('.chart-data tbody tr');
  const initialCount = await rows.count();
  await page.locator('#toggles button').first().click();
  await expect(page.locator('#toggles button').first()).toHaveAttribute('aria-pressed', 'false');
  await expect(rows).toHaveCount(initialCount - 1);
  await page.getByRole('button', { name: 'Show all', exact: true }).click();
  await expect(rows).toHaveCount(initialCount);
  await page.selectOption('#run', { index: await page.locator('#run option').count() > 1 ? 1 : 0 });
  await expect(page.locator('#chart-status')).toBeHidden();
  await expect(workloads).toHaveCount(1);
  await expect(scrollWorkload).toHaveAttribute('data-metric', 'renderer_fps');
  await expect(page.locator('.echart svg').first()).toBeVisible();
  for (const button of await page.locator('#toggles button').all()) await button.click();
  await expect(page.locator('#charts')).toContainText('No measured frameworks are visible');
  await expect(page.locator('#charts .echart svg')).toHaveCount(0);
  await page.getByRole('button', { name: 'Show all', exact: true }).click();
  await expect(page.locator('.echart svg').first()).toBeVisible();
  expect(errors).toEqual([]);
});

test('latest collection includes both new variants and Blinc measurements', async ({ page }) => {
  const dashboard = await (await page.request.get('/data/benchmarks.json')).json();
  const latest = dashboard.runs[0];
  expect(latest.measurement).toBe('mangohud-scroll-v2');
  await page.goto('/');
  await expect(page.locator('#run')).toHaveValue(latest.id);
  for (const name of ['Dioxus Native Vello Hybrid', 'Whisker', 'Freya (Optimized)']) {
    await expect(page.locator('#toggles').getByRole('button', { name, exact: true })).toBeVisible();
  }
  const plot = page.locator('[data-scenario="scroll"] .echart');
  await expect(plot).toContainText('Whisker');
  await expect(plot).toContainText('Dioxus Native Vello Hybrid');
  const group = latest.groups.renderer_fps[0];
  expect(group.boundary).toBe('mangohud-present-observation');
  const blinc = group.points.find(point => point.framework === 'blinc');
  expect(blinc.value).toBeGreaterThan(0);
  await expect(plot.locator('svg text').filter({ hasText: /^Blinc \d/ })).toHaveCount(1);
  await page.locator('.chart-data > summary').click();
  await expect(page.locator('.chart-data tbody tr')).toHaveCount(latest.frameworks.length);
  for (const missing of group.missing) {
    const name = latest.frameworks.find(framework => framework.id === missing.framework).name;
    await expect(plot).toContainText(`${name}: no data`);
    await expect(plot.locator('svg text').filter({ hasText: new RegExp(`^${name} \\d`) })).toHaveCount(0);
    await expect(page.locator('.chart-data tbody tr').filter({ has: page.getByRole('rowheader', { name, exact: true }) })).toContainText('Unavailable');
    const toggle = page.locator('#toggles').getByRole('button', { name, exact: true });
    await toggle.click();
    await expect(plot).not.toContainText(`${name}: no data`);
    await toggle.click();
    await expect(plot).toContainText(`${name}: no data`);
  }
});

test('compact plots share a workload row with readable inline labels', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('#run')).toBeEnabled();
  await selectScrollCollection(page);
  const plot = page.locator('[data-scenario="scroll"] .echart').first();
  await expect(plot).toContainText('Workload');
  const labels = plot.locator('svg text').filter({ hasText: /^Freya \d/ });
  await expect(labels).toHaveCount(1);
  const bounds = await plot.boundingBox();
  expect(bounds.height).toBeLessThan(400);
  const boxes = await plot.locator('svg text').evaluateAll(nodes => nodes
    .filter(node => /\s\d[\d,.]*$/.test(node.textContent))
    .map(node => {
      const box = node.getBoundingClientRect();
      const marker = node.previousElementSibling.getBoundingClientRect();
      return { text: node.textContent, left: box.left, right: box.right, top: box.top, bottom: box.bottom, markerLeft: marker.left };
    }));
  expect(boxes.length).toBeGreaterThan(5);
  expect(boxes.find(box => box.text.startsWith('Blinc ')).markerLeft).toBeLessThan(bounds.x + 320);
  for (let index = 0; index < boxes.length; index++) {
    expect(boxes[index].right, `${boxes[index].text} is left of its marker`).toBeLessThan(boxes[index].markerLeft);
    expect(boxes[index].left).toBeGreaterThanOrEqual(bounds.x + 205);
    for (const other of boxes.slice(index + 1)) {
      const box = boxes[index];
      const overlaps = box.left < other.right && box.right > other.left && box.top < other.bottom && box.bottom > other.top;
      expect(overlaps, `${box.text} overlaps ${other.text}`).toBe(false);
    }
  }
  await labels.hover();
  await expect(plot).toContainText('Median:');
});

test('redesign uses monochrome styling with green accents and Whisker last', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#run')).toBeEnabled();
  await expect(page.locator('h1')).toHaveText('are we actually GUI yet? 🚧 experimental');
  await expect(page.locator('#bench-heading')).toHaveText('Benchmarks');
  await expect(page.locator('#features-heading')).toHaveText('Features');
  await expect(page.locator('#features thead th').last()).toHaveText('Whisker');
  await expect(page.locator('#features thead th').nth(-2)).toHaveText('Blinc');
  await expect(page.locator('#features thead th').nth(1)).not.toHaveText('Blinc');
  await expect(page.locator('.hero, .stats, .pill, .chart-footnote')).toHaveCount(0);
  const theme = await page.evaluate(() => {
    const style = getComputedStyle(document.documentElement);
    return { accent: style.getPropertyValue('--accent').trim(), font: style.fontFamily, background: style.backgroundColor };
  });
  expect(theme.accent).toBe('#7aef83');
  expect(theme.font).toContain('monospace');
  expect(theme.background).toBe('rgb(11, 11, 11)');
  await expect(page.locator('.feature-status .status-mark').first()).toHaveCSS('width', '28px');
  await expect(page.locator('.collection-info')).toHaveCount(0);
  await expect(page.locator('.machine-specs')).toHaveText('AMD Ryzen 5 5600X · AMD 9060XT 16GB · 144hz');
});

test('feature rectangles distinguish support levels and retain evidence', async ({ page }) => {
  await page.goto('/');
  const markers = page.locator('#features tbody .status-mark');
  await expect(markers).toHaveCount(490);
  await expect(markers.filter({ hasText: /[●◇◐≈○]/ })).toHaveCount(0);
  await expect(page.locator('tbody .status-mark[data-status="yes"]').first()).toHaveCSS('background-color', 'rgb(122, 239, 131)');
  const unsupported = page.locator('tbody .status-mark[data-status="no"]').first();
  await expect(unsupported).toHaveCSS('background-color', 'rgb(239, 68, 68)');
  await expect(unsupported).toHaveCSS('background-image', 'none');
  await expect(page.locator('tbody .status-mark[data-status="different"]').first()).toHaveCSS('background-color', 'rgb(255, 204, 0)');
  await expect(page.locator('tbody .status-mark[data-status="partial"]').first()).toHaveCSS('background-image', /repeating-linear-gradient/);
  const unknown = page.locator('tbody .status-mark[data-status="unknown"]').first();
  await expect(unknown).toHaveCSS('background-image', 'none');
  await expect(unknown).toHaveCSS('background-color', 'rgb(112, 112, 106)');
  await expect(unknown).toHaveCSS('color', 'rgb(0, 0, 0)');
  await expect(unknown).toHaveText('?');
  const summary = unknown.locator('..');
  await expect(summary).toHaveAccessibleName(/Unknown/);
  await summary.click();
  await expect(summary.locator('..').getByRole('link', { name: 'Source', exact: true })).toBeVisible();
});

test('feature evidence is usable without JavaScript', async ({ browser }) => {
  const context = await browser.newContext({ javaScriptEnabled: false });
  const page = await context.newPage();
  await page.goto('/');
  await expect(page.locator('#features tbody tr')).toHaveCount(49);
  await expect(page.locator('.downloads-row img')).toHaveCount(13);
  await expect(page.locator('.downloads-row img').first()).toHaveAttribute('src', /^https:\/\/img\.shields\.io\/crates\/dr\/.+\.svg\?label=recent$/);
  await expect(page.locator('.downloads-row')).toContainText('Not published');
  const evidence = page.locator('#features td details').first();
  await evidence.locator('summary').click();
  await expect(evidence).toHaveAttribute('open', '');
  await expect(evidence.getByRole('link', { name: 'Source', exact: true })).toBeVisible();
  await context.close();
});

test('binary sizes show all release executables independently of benchmark collections', async ({ page }) => {
  const errors = [];
  page.on('pageerror', error => errors.push(error.message));
  await page.emulateMedia({ reducedMotion: 'reduce' });
  const snapshot = await (await page.request.get('/data/binary-sizes.json')).json();
  expect(snapshot.results).toHaveLength(16);
  await page.goto('/');
  const section = page.locator('#binary-sizes');
  const plot = section.locator('.echart');
  await expect(plot.locator('svg')).toBeVisible();
  await expect(plot).toContainText('Binary size (MiB)');
  await expect(plot).toContainText('Artifact');
  await expect(plot).not.toContainText('undefined');
  await section.locator('summary').click();
  await expect(section.locator('tbody tr')).toHaveCount(snapshot.results.length);
  for (const result of snapshot.results) {
    const row = section.getByRole('row').filter({ has: page.getByRole('rowheader', { name: result.name, exact: true }) });
    await expect(row).toContainText(result.bytes.toLocaleString('en-US'));
    expect(snapshot.group.points.find(point => point.framework === result.framework).value).toBe(result.bytes / 1048576);
  }
  const labels = plot.locator('svg text').filter({ hasText: /^Freya \d/ });
  await labels.hover();
  await expect(plot).toContainText('Size:');
  await expect(plot).not.toContainText('Median:');
  await expect(page.locator('#run')).toBeEnabled();
  await page.selectOption('#run', { index: await page.locator('#run option').count() > 1 ? 1 : 0 });
  await page.locator('#toggles button').first().click();
  await expect(section.locator('tbody tr')).toHaveCount(16);
  await expect(labels).toHaveCount(1);
  await page.setViewportSize({ width: 390, height: 844 });
  const dimensions = await page.evaluate(() => ({ width: innerWidth, scroll: document.documentElement.scrollWidth }));
  expect(dimensions.scroll).toBeLessThanOrEqual(dimensions.width);
  expect(await section.locator('.compact-chart-frame').evaluate(element => element.scrollWidth > element.clientWidth)).toBe(true);
  expect(errors).toEqual([]);
});

test('mobile layout keeps charts within the viewport', async ({ page }) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.goto('/');
  await expect(page.locator('#run')).toBeEnabled();
  await expect(page.locator('.echart svg').first()).toBeVisible();
  const dimensions = await page.evaluate(() => ({ width: window.innerWidth, scroll: document.documentElement.scrollWidth }));
  expect(dimensions.scroll).toBeLessThanOrEqual(dimensions.width);
  const frame = page.locator('.compact-chart-frame').first();
  const chart = await frame.boundingBox();
  expect(chart.x + chart.width).toBeLessThanOrEqual(dimensions.width);
  const scroll = await frame.evaluate(element => {
    element.scrollLeft = element.scrollWidth;
    return element.scrollLeft;
  });
  expect(scroll).toBeGreaterThan(0);
});
