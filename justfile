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

# Open a standalone scroll example without measurement or simulated input.
app framework="freya":
    cargo run --release --locked --manifest-path "apps/$1/Cargo.toml"

# List the standalone GUI examples.
list:
    python3 apps/bench.py list

# Build a standalone scroll example.
bench-build framework="freya":
    cargo build --release --locked --manifest-path "apps/$1/Cargo.toml"

# Run external MangoHud captures, saving history only with --save.
bench *args:
    python3 apps/bench.py run "$@"

# Collect release executable sizes for the website without launching apps.
binary-sizes *args:
    python3 apps/sizes.py "$@"

# Check Python compilation and build the website without running benchmarks.
check:
    python3 -m py_compile apps/bench.py apps/frames.py apps/scroll.py apps/sizes.py
    npm run build
