# Astro + Apache ECharts website

The production static website for Are we actually GUI yet. Requires Node.js 22.12 or newer.

## Run

From the repository root:

```sh
npm --prefix site ci
npm run dev
npm run build
npm run serve
```

Or run `npm ci`, `npm run dev`, `npm run build`, and `npm run preview` directly inside `site/`. Production output is `site/dist/`.

## Structure

- `src/pages/` and `src/components/` contain Astro components.
- `src/scripts/` contains the ECharts explorer and compact chart renderer.
- `src/styles/` contains the website styles.
- `src/lib/` contains data loading, schema validation, and chart transformation helpers.
- `data/` contains the source-backed feature claims, library metadata, and historical benchmark collections.
- `public/` contains static public assets.

Astro generates the page and feature table at build time. Native HTML details expose feature evidence without client hydration. Workloads are rendered one below the other, each with an automatically selected measurement. Collection selection and framework filters apply to every workload chart. The charts include inline labels, repetition ranges, tooltips, horizontal selection zoom, reset, and SVG export.

Dependencies are version-pinned and bundled locally. No React, server adapter, WASM, runtime backend, or CDN dependency is required.

## Measurement integrity

The site validates its local source data and precomputes chart groups without shipping raw traces to the browser. Scenario versions, renderer-boundary exclusions, framework versions, repetition ranges, and measurement caveats are preserved.

Only scrolling and startup workloads are retained. Scrolling prefers present-call cadence and startup uses startup time. Collections without cadence data show labeled CPU measurements instead. Unrelated CPU measurements are not substituted for startup.

Dots are collection medians. Lines show repetition minima and maxima, not confidence intervals. Earlier renderer stages do not enter present-call charts. Startup v1 remains explicitly marked as unsuitable for comparison. Resource use is not presented as a speed ranking.

`/data/benchmarks.json` is the derived explorer payload. Source data remains in `data/`, including historical collection provenance. It is not rewritten or recollected by a site build. MangoHud history from the standalone examples has a different schema and is not automatically imported.

## Vercel deployment

The root `vercel.json` supports deployment with the repository root as the project root. It installs with `npm --prefix site ci`, builds with `npm run build`, and publishes `site/dist/`.

Alternatively, set the Vercel Root Directory to `site` and use its local Astro configuration: `npm ci`, `npm run build`, and output directory `dist`. All site dependencies and data are inside that folder. No Vercel adapter is required. Deployment has not been performed as part of this migration.

## Checks

```sh
npm --prefix site run test:unit
npm run build
```

Browser checks remain available:

```sh
cd site
npx playwright install chromium
npm run test:browser
```

To use an installed Chromium executable, set `CHROMIUM_PATH`. These checks cover chart rendering and labels, tooltips, filtering, reset states, stacked workload sections and collection changes, mobile scrolling, and feature evidence without JavaScript.
