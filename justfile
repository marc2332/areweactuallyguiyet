set positional-arguments

default:
    @just --list

# Build the website and serve it at http://127.0.0.1:8000.
site: build-site
    node scripts/serve.mjs

# Build and validate the static website.
build-site:
    node scripts/build.mjs

# Serve an already-built website.
serve port="8000":
    node scripts/serve.mjs "$1"

# Open an interactive native application without collecting measurements.
app framework="egui" scenario="scroll":
    node scripts/launch.mjs "$1" "$2"

# Check local desktop and measurement prerequisites.
doctor:
    cargo run -p gui-bench-runner -- doctor

# Show adapter and scenario availability.
list:
    cargo run -p gui-bench-runner -- list

# Build a release benchmark executable without running it.
bench-build framework="egui" scenario="scroll":
    cargo run --release -p gui-bench-runner -- build --framework "$1" --scenario "$2"

# Collect release measurements, with optional extra runner flags.
bench framework="egui" scenario="scroll" machine="desktop" *args:
    #!/usr/bin/env bash
    set -euo pipefail
    framework="$1"
    scenario="$2"
    machine="$3"
    shift 3
    cargo run --release -p gui-bench-runner -- run --framework "$framework" --scenario "$scenario" --machine "$machine" "$@"

# Run every adapter and workload.
bench-all machine="desktop" *args:
    @just bench freya,freya-optimized,egui,vizia,iced,gpui,gpui-fast,dioxus-native,dioxus-native-vello,dioxus-native-vello-hybrid,slint,floem,makepad,rune,zgui startup,scroll,virtual-scroll,switches,memory "$@"

# Exercise the full runner with a short development-only real-window run.
smoke framework="egui" scenario="scroll":
    @just bench "$1" "$2" smoke --profile dev --repetitions 1 --warmup 1 --duration 3 --output /tmp/gui-bench-smoke

# Import a run and rebuild the website, then refresh the browser.
collect path:
    node scripts/collect.mjs "$1"
    node scripts/build.mjs

# Import the latest run for a machine and rebuild the website.
collect-latest machine="desktop":
    #!/usr/bin/env bash
    set -euo pipefail
    shopt -s nullglob
    runs=("results/$1"/*/run.json)
    if (( ${#runs[@]} == 0 )); then
        printf 'No benchmark runs found for machine %s\n' "$1" >&2
        exit 1
    fi
    just collect "${runs[${#runs[@]} - 1]}"

# Check runner compilation and website generation without running benchmarks.
check:
    cargo check --locked -p gui-bench-runner
    node scripts/build.mjs
