#!/usr/bin/env python3
"""Build, capture, inspect, and publish standalone GUI benchmarks."""
import argparse
import json
import math
from pathlib import Path
import subprocess
import sys
import tempfile

from benchmark import dashboard, publish, runner, sizes
from benchmark.frames import FrameLog, parse_csv


def positive_number(value):
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("must be finite and positive")
    return number


def nonnegative_integer(value):
    number = int(value)
    if number < 0:
        raise argparse.ArgumentTypeError("must be nonnegative")
    return number


def positive_integer(value):
    number = nonnegative_integer(value)
    if number == 0:
        raise argparse.ArgumentTypeError("must be positive")
    return number


def history_options(parser):
    parser.add_argument("--save", action="store_true", help="Append to history, otherwise print only")
    parser.add_argument("--history", type=Path)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="List available apps")
    for benchmark in ("scroll", "dashboard"):
        capture = commands.add_parser(benchmark, help=f"Build and capture the {benchmark} workload")
        capture.set_defaults(benchmark=benchmark)
        capture.add_argument("apps", nargs="*", help="App IDs, or all (default)")
        if benchmark == "scroll":
            capture.add_argument("--rows", type=positive_integer, nargs="+", default=[1000, 3000, 5000, 7000])
            capture.add_argument("--duration", type=positive_number, default=15)
        capture.add_argument("--warmup", type=positive_number, default=2)
        capture.add_argument("--rate", type=positive_number, help="Input events per second")
        capture.add_argument("--fps-limit", type=nonnegative_integer, default=144 if benchmark == "scroll" else 0)
        capture.add_argument("--vsync", action=argparse.BooleanOptionalAction, default=benchmark == "scroll",
                             help="Keep application-default synchronization instead of requesting it disabled")
        capture.add_argument("--repetitions", type=positive_integer, default=1)
        capture.add_argument("--startup-timeout", type=positive_number, default=30)
        capture.add_argument("--output", type=Path, default=runner.ROOT / "results/apps")
        capture.add_argument("--no-build", action="store_true", help="Reuse cached benchmark binaries")
        history_options(capture)

    parse = commands.add_parser("parse", help="Summarize an existing MangoHud CSV")
    parse.add_argument("benchmark", choices=("scroll", "dashboard"))
    parse.add_argument("app", help="App ID")
    parse.add_argument("csv", type=Path)
    parse.add_argument("--duration", type=positive_number, help="Required scroll measurement duration")
    parse.add_argument("--start", type=float, help="Earliest scroll measurement start in CSV seconds")
    history_options(parse)

    size = commands.add_parser("sizes", help="Build production binaries and collect executable sizes")
    size.add_argument("--no-build", action="store_true")
    size.add_argument("--output", type=Path, default=runner.ROOT / "site/data/binary-sizes-snapshot.json")

    publication = commands.add_parser("publish", help="Convert a run.json capture to website data")
    publication.add_argument("run", type=Path)
    publication.add_argument("--history", type=Path, default=runner.ROOT / "site/data/history.json")
    publication.add_argument("--save", action="store_true", help="Write website history, otherwise print only")
    publication.add_argument("--latest", action="store_true", help="Update the latest collection, preserving other workloads")

    arguments = parser.parse_args(argv)
    if arguments.command == "parse":
        if arguments.benchmark == "scroll" and arguments.duration is None:
            parser.error("scroll parsing requires --duration")
        if arguments.benchmark == "dashboard" and (arguments.duration is not None or arguments.start is not None):
            parser.error("dashboard parsing uses the complete capture, without --duration or --start")
    if arguments.command == "publish" and arguments.latest and not arguments.save:
        parser.error("--latest requires --save")
    if arguments.command == "scroll":
        if len(set(arguments.rows)) != len(arguments.rows) or any(rows > 2**31 - 1 for rows in arguments.rows):
            parser.error("--rows must contain unique positive integers no larger than 2147483647")
    if arguments.command in ("scroll", "dashboard", "parse") and arguments.history is None:
        measurement = dashboard.MEASUREMENT if arguments.benchmark == "dashboard" else runner.MEASUREMENT
        arguments.history = runner.DIRECTORY / (measurement.removeprefix("mangohud-") + "-history.json")

    try:
        apps = runner.load_apps()
        if arguments.command == "list":
            for app in apps.values():
                print(f"{app['id']:20} {app['renderer']:8} {app['title']}")
        elif arguments.command in ("scroll", "dashboard"):
            arguments.no_vsync = not arguments.vsync
            if arguments.rate is None:
                arguments.rate = 240 if arguments.benchmark == "dashboard" else arguments.fps_limit or 144
            return runner.run_benchmarks(arguments, apps)
        elif arguments.command == "sizes":
            with tempfile.TemporaryDirectory(prefix="gui-binary-sizes-") as directory:
                snapshot = sizes.collect(apps, Path(directory), arguments.no_build)
            arguments.output.parent.mkdir(parents=True, exist_ok=True)
            arguments.output.write_text(json.dumps(snapshot, indent=2, allow_nan=False) + "\n")
            print(f"Saved {len(snapshot['results'])} executable sizes to {arguments.output}")
        elif arguments.command == "publish":
            libraries = json.loads((runner.ROOT / "site/data/libraries.json").read_text())["libraries"]
            run = publish.website_run(json.loads(arguments.run.read_text()), apps, libraries)
            if arguments.save:
                publish.save(arguments.history, run, latest=arguments.latest)
                print(f"Published to {arguments.history}")
            else:
                print(json.dumps(run, indent=2, allow_nan=False))
        else:
            if arguments.app not in apps:
                raise ValueError(f"Unknown app: {arguments.app}")
            if arguments.benchmark == "dashboard":
                with FrameLog(arguments.csv) as trace:
                    trace.finish()
                    metrics = dashboard.summarize(trace.frames)
            else:
                metrics = parse_csv(arguments.csv, arguments.duration, arguments.start)
            run = runner.new_run(list(argv) if argv is not None else sys.argv[1:], arguments.benchmark)
            run["results"].append({
                "app": arguments.app, "status": "ok", "source": "imported-csv",
                "csv": str(arguments.csv.resolve()),
                "capture_configuration": "unknown, must be a log_interval=0 per-frame capture",
                "metrics": metrics,
            })
            runner.finish(run, arguments)
        return 0
    except (OSError, ValueError, KeyError, StopIteration, RuntimeError, subprocess.SubprocessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("Interrupted, history not saved", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
