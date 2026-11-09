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
just bench-hover-all desktop --fast --duration 4
just smoke egui scroll
just collect-latest desktop
```

Site design adapted from [Are we layout yet?](https://github.com/marc2332/arewelayoutyet).
