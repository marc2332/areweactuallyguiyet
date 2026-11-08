# Are we actually GUI yet?

Rust GUI benchmarks and feature comparisons.

## Website

[Visit the website](https://areweactuallyguiyet.vercel.app/).

Requires Node.js. 

```sh
just site              # Build and serve
just build-site        # Build dist/
just serve 8000        # Serve an existing build
```

## Benchmarks

```sh
just doctor
just list
just app egui scroll
just bench egui scroll desktop --repetitions 3 --output /tmp/gui-bench
just bench freya,freya-optimized scroll desktop --output /tmp/gui-bench
just smoke egui scroll
just collect-latest desktop
```

The website selector shows published full collections from `site/data/history.json`, newest first. Individual runs are temporary inputs and cannot be published with `just collect`. A full collection must include every adapter and scenario in `apps/catalog.json`. Builds do not automatically import local or temporary results.

Site design adapted from [Are we layout yet?](https://github.com/marc2332/arewelayoutyet).
