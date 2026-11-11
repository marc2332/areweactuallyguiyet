set positional-arguments

default:
    @just --list

# Run the Astro development server.
site port="4321":
    npm --prefix site run dev -- --port "$1"

# Build the static Astro website.
build-site:
    npm run build

# Preview the production website.
serve port="4321":
    npm --prefix site run preview -- --port "$1"

# Open a standalone benchmark without measurement or simulated input.
app framework="freya" benchmark="scroll":
    cargo run --release --locked --manifest-path "apps/$1/Cargo.toml" --no-default-features --features "$2$(if [ "$1" = dioxus-native ]; then printf ',skia'; fi)"

# List the standalone GUI examples.
list:
    python3 apps/bench.py list

# Run scroll benchmarks, saving history only with --save.
bench *args:
    python3 apps/bench.py scroll "$@"

# Build production executables and collect their sizes for the website without launching apps.
binary-sizes *args:
    python3 apps/bench.py sizes "$@"

# Check Python compilation and build the website without running benchmarks.
check:
    python3 -m compileall -q apps/bench.py apps/benchmark
    npm run build
