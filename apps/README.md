# Standalone Rust GUI benchmarks

Fourteen independent GUI apps render the same non-virtualized list of 2,000 rows. Each row is 960 × 48 logical pixels, with 12 px padding, black 14 px `Item N` text, and alternating `#f5f5f5` / `#e6e6e6` backgrounds. Font families remain toolkit-native.

The apps contain no benchmark callbacks, timers, forced redraws, or programmatic scrolling. Dependencies are pinned in each app's manifest and lockfile.

```sh
python3 apps/bench.py list
cargo run --release --locked --manifest-path apps/freya/Cargo.toml
```

## MangoHud scrolling

```sh
python3 apps/bench.py run all
python3 apps/bench.py run freya --no-build --duration 15 --repetitions 3
python3 apps/bench.py run all --fps-limit 0 --no-vsync
python3 apps/bench.py run freya --save
```

Requires MangoHud, X11/XWayland, `xwininfo`, `xprop`, and the X11 input libraries. The runner launches each app through MangoHud and sends continuous wheel input. Defaults are a 144 FPS limit, two-second warmup, and 15-second measurement. Input frequency follows the FPS limit, or 144 Hz when uncapped. Override with `--rate`. `--no-vsync` requests disabled synchronization through MangoHud, but does not verify the actual presentation mode.

Recording begins during input pre-roll. After warmup the measurement anchors to a fresh validated CSV frame and requires a frame bracketing the full endpoint. The exact window retains stalls and its trailing silent period. Corrupt clocks, malformed rows, and truncated captures fail rather than produce normalized estimates. No valid frame in a fully bracketed window means zero FPS, not an omitted result.

```sh
python3 apps/bench.py parse /path/to/capture.csv --app freya --duration 15
```

CSV parsing expects `log_interval=0` and elapsed nanoseconds. `--start` selects the first frame at or after the requested elapsed seconds. Imported captures do not claim automated input or warmup verification. MangoHud summary CSVs are not valid per-frame inputs.

Reported FPS counts present observations in the exact timestamp window. It is not confirmed displayed FPS, distinct content changes, CPU render execution time, or GPU execution time. Intervals include waiting, synchronization, and the limiter. Overlay rendering and synchronous logging add overhead. MangoHud's own summary includes pre-roll and is not the runner's measurement-window summary.

Scroll history defaults to `apps/history.json`, with identity `mangohud-scroll-v2`. Its schema is not automatically combined with the website's historical collections.

## Persistence and artifacts

Neither command opens or changes history without `--save`. `--history` alone only selects a destination. Explicit saves use locking and atomic replacement and reject incompatible measurement identities. Artifacts and `run.json` are written under `results/apps/<run-id>/` regardless of saving. Application logs and failures remain available there. Interrupted runs do not save history.

## Checks

```sh
python3 -m py_compile apps/bench.py apps/frames.py apps/scroll.py
python3 -m unittest discover -s apps/tests -p 'test_bench.py'
```

Original row-style screenshots and validation artifacts remain under `results/apps/row-style-validation/`. Earlier version-1 scrolling captures are diagnostic only and are not comparable to corrected version-2 captures. Blinc logs with impossible frametimes are rejected rather than published.
