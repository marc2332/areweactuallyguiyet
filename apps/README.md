# Standalone Rust GUI benchmarks

Each app selects a benchmark through Cargo features. `scroll` is the default, `dashboard` is opt-in, and selecting both or neither is rejected. The implementations live in `src/scroll.rs` and `src/dashboard.rs`, selected by `main.rs`. Renderer variants can share the base app's implementation.

```sh
python3 apps/bench.py list
cargo run --release --locked --manifest-path apps/freya/Cargo.toml
cargo run --release --locked --manifest-path apps/freya/Cargo.toml --no-default-features --features dashboard
just app freya dashboard
```

## Scrolling

The scroll UI is a non-virtualized list of 2,000 rows. Each row is 960 × 48 logical pixels, with 12 px padding, black 14 px `Item N` text, and alternating `#f5f5f5` / `#e6e6e6` backgrounds. Font families remain toolkit-native.

```sh
python3 apps/bench.py run all
python3 apps/bench.py run freya --no-build --duration 15 --repetitions 3
python3 apps/bench.py run all --fps-limit 0 --no-vsync
python3 apps/bench.py run freya --save
```

The runner sends continuous wheel input. Defaults are a 144 FPS limit, two-second warmup, and 15-second measurement. Input frequency follows the FPS limit, or 144 Hz when uncapped. Override with `--rate`. `--no-vsync` requests disabled synchronization through MangoHud, but does not verify the actual presentation mode.

Recording begins during input pre-roll. After warmup the measurement anchors to a fresh validated CSV frame and requires a frame bracketing the full endpoint. The exact window retains stalls and its trailing silent period. Corrupt clocks, malformed rows, and truncated captures fail rather than produce normalized estimates. No valid frame in a fully bracketed window means zero FPS, not an omitted result.

```sh
python3 apps/bench.py parse /path/to/capture.csv --app freya --duration 15
```

CSV parsing expects `log_interval=0` and elapsed nanoseconds. `--start` selects the first frame at or after the requested elapsed seconds. Imported captures do not claim automated input or warmup verification. MangoHud summary CSVs are not valid per-frame inputs.

Reported FPS counts presentation observations in the exact timestamp window. It is not confirmed displayed FPS, distinct content changes, CPU render execution time, or GPU execution time. Intervals include waiting, synchronization, and the limiter. Overlay rendering and synchronous logging add overhead. MangoHud's own summary includes pre-roll and is not the runner's measurement-window summary.

Scroll history defaults to `apps/history.json`, with identity `mangohud-scroll-v2`.

## Dashboard

The dashboard has a fixed toolbar with a text input, a Refresh button with a visible counter, an enabled toggle, and a scrollable list of 1,000 project records with text and images. Toolkit-native styling is retained. The client area is 1000 × 700 logical pixels. The runner scales input coordinates to the physical client area for desktop scaling.

```sh
python3 apps/bench.py run freya --benchmark dashboard
python3 apps/bench.py run all --benchmark dashboard --repetitions 3 --save
python3 apps/bench.py run egui --benchmark dashboard --rate 240
python3 apps/bench.py parse /path/to/dashboard.csv --app egui --benchmark dashboard
```

The runner focuses and clears the input before warmup. The measured workload then runs four consecutive two-second phases: type/delete text, click Refresh, toggle enabled, and scroll down/up. Total input time is eight seconds, with no pauses between phases. Warmup is separate. Input is paced at a requested 240 events/s by default, configurable with `--rate`, with actual sent events recorded per phase. Dashboard defaults to no FPS limiter and requested disabled VSync. Use `--fps-limit` or `--vsync` to override. `--duration` applies only to scroll captures. There are no app timers or forced redraws.

The main dashboard metric is the worst presentation gap anywhere in the eight-second workload. Logging starts before measurement, a validated frame anchors the start, and capture continues until a frame brackets the endpoint. Gaps crossing phase transitions and trailing silence through the endpoint are included. Pre-workload idle is excluded. Repetitions are combined by taking the worst gap, not their median. Older captures without a bracketed workload window cannot supply this metric.

Dashboard captures also sample application process RSS from `/proc/<pid>/statm` every 50 ms during the workload. The memory graph connects the lowest sample, arithmetic mean of all samples, and highest sample across successful repetitions. It excludes startup, child processes, and GPU allocations. Older captures have no memory measurements.

Dashboard results also retain per-frame intervals, median, and p95 as diagnostic information. They deliberately do not report FPS or frame counts as performance scores. These intervals include idle time between actions, synchronization, and limiting. They measure presentation cadence during dashboard interactions, not CPU/GPU execution durations or input latency. Action timestamps and phase boundaries describe input dispatch, not verified widget updates or exact phase alignment in the MangoHud clock. Visually inspect an app before comparing its captures.

Dashboard history defaults to `apps/dashboard-v3-history.json`, with identity `mangohud-dashboard-v3`. Version 3 uses 1,000 records instead of 100 to avoid reaching the bottom during the scrolling phase. It cannot be mixed with scroll history or earlier dashboard captures. Cached dashboard binaries are versioned, so rebuild before using `--no-build`. To add dashboard results to the latest website collection without replacing its scroll results:

```sh
python3 apps/publish.py results/apps/<run-id>/run.json --save --latest
```

## Build and capture requirements

Requires MangoHud, X11/XWayland, `xwininfo`, `xprop`, and the X11 input libraries. Dependencies are pinned in each app's manifest and lockfile. The runner builds with `--no-default-features --features <benchmark>` plus the app's default renderer features, and retains a benchmark-specific executable copy. Direct Dioxus Native commands also need `skia`, for example `--features dashboard,skia`. `--no-build` uses those copies, so first build each app and benchmark with `bench.py`, not just `cargo build`.

## Binary sizes

```sh
just binary-sizes
```

This rebuilds all 16 scroll apps with `cargo build --profile production --locked` and updates `site/data/binary-sizes-snapshot.json`. Each app defines this profile:

```toml
[profile.production]
inherits = "release"
opt-level = 3
debug = false
lto = true
codegen-units = 1
strip = true
incremental = false
```

Sizes measure the resulting executable without additional stripping or compression. Shared libraries and external assets are excluded. Production executable copies are separate from release runtime-benchmark copies. `just binary-sizes --no-build` reuses only production copies previously built by this command.

## Persistence and artifacts

Neither command opens or changes history without `--save`. `--history` alone only selects a destination. Explicit saves use locking and atomic replacement and reject incompatible measurement identities. Artifacts and `run.json` are written under `results/apps/<run-id>/` regardless of saving. Application logs and failures remain available there. Interrupted runs do not save history.

The two measurement schemas are not automatically combined with the website's historical collections. Original row-style screenshots and validation artifacts remain under `results/apps/row-style-validation/`. Earlier version-1 scrolling captures are diagnostic only and are not comparable to corrected version-2 captures. Blinc logs with impossible frametimes are rejected rather than published.

## Checks

```sh
python3 -m py_compile apps/bench.py apps/frames.py apps/scroll.py apps/dashboard.py
python3 -m unittest discover -s apps/tests -p 'test_dashboard.py'
```
