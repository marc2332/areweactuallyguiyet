#!/usr/bin/env python3
import argparse
import contextlib
import datetime
import fcntl
import json
import os
from pathlib import Path
import platform
import re
import shutil
import socket
import subprocess
import sys
import tempfile
import time
import tomllib
import uuid

import dashboard
from frames import CaptureWindow, FrameLog, parse_csv, summarize
from scroll import WheelInput, positive_number, window_client_pid

DIRECTORY = Path(__file__).resolve().parent
ROOT = DIRECTORY.parent
MEASUREMENT = "mangohud-scroll-v2"


def load_apps():
    apps = {}
    for path in sorted(DIRECTORY.glob("*/app.json")):
        app = json.loads(path.read_text())
        if app["id"] != path.parent.name or app["renderer"] not in ("opengl", "vulkan"):
            raise ValueError(f"Invalid app metadata: {path}")
        apps[app["id"]] = {**app, "manifest": path.parent / "Cargo.toml"}
    return apps


def save_history(path, run):
    measurement = run.get("measurement")
    if measurement not in (MEASUREMENT, dashboard.MEASUREMENT):
        raise ValueError(f"Not an apps measurement identity: {measurement}")
    path = Path(path).resolve()
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.with_name(path.name + ".lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        history = json.loads(path.read_text()) if path.exists() else {
            "schemaVersion": 1, "measurement": measurement, "runs": [],
        }
        if (not isinstance(history, dict) or history.get("schemaVersion") != 1
                or history.get("measurement") != measurement
                or not isinstance(history.get("runs"), list)
                or any(not isinstance(previous, dict) or previous.get("measurement") != measurement
                       for previous in history.get("runs", []))):
            raise ValueError(f"Not an apps {measurement} history file: {path}")
        history["runs"].append(run)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as file:
                temporary = Path(file.name)
                json.dump(history, file, indent=2, allow_nan=False)
                file.write("\n")
            temporary.replace(path)
        finally:
            if temporary and temporary.exists():
                temporary.unlink()


def finish(run, arguments):
    print(json.dumps(run, indent=2, allow_nan=False))
    if arguments.save:
        save_history(arguments.history, run)
        print(f"Saved to {arguments.history}", file=sys.stderr)


def new_run(command, benchmark="scroll"):
    return {
        "id": str(uuid.uuid4()),
        "timestamp": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "command": command,
        "measurement": dashboard.MEASUREMENT if benchmark == "dashboard" else MEASUREMENT,
        "benchmark": benchmark,
        "system": {"os": platform.platform(), "machine": platform.machine()},
        "results": [],
    }


def build(app, output, skip_build, benchmark="scroll"):
    manifest = str(app["manifest"])
    defaults = tomllib.loads(app["manifest"].read_text()).get("features", {}).get("default", [])
    features = ",".join([benchmark, *(feature for feature in defaults if feature not in ("scroll", "dashboard"))])
    if not skip_build:
        print(f"Building {app['id']} ({benchmark})...", file=sys.stderr)
        with (output / f"build-{app['id']}.log").open("w") as log:
            subprocess.run(["cargo", "build", "--release", "--locked", "--manifest-path", manifest,
                            "--no-default-features", "--features", features],
                           stdout=log, stderr=subprocess.STDOUT, check=True)
    metadata = json.loads(subprocess.check_output([
        "cargo", "metadata", "--no-deps", "--format-version", "1", "--locked", "--manifest-path", manifest,
    ], text=True))
    package = next(package for package in metadata["packages"] if Path(package["manifest_path"]) == app["manifest"])
    binaries = [target["name"] for target in package["targets"] if "bin" in target["kind"]]
    if len(binaries) != 1:
        raise ValueError(f"Expected exactly one binary for {app['id']}")
    binary = Path(metadata["target_directory"]) / "release" / binaries[0]
    selected = Path(metadata["target_directory"]) / "benchmarks" / benchmark / app["id"] / binary.name
    if not skip_build:
        selected.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(binary, selected)
    if not selected.is_file():
        raise ValueError(f"Missing {benchmark} release binary: {selected}. Build it with bench.py first")
    return selected


def check_alive(process):
    if process.poll() is not None:
        raise RuntimeError(f"App exited with status {process.returncode}, see application.log")


def wait_for_window(process, title, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        check_alive(process)
        try:
            tree = subprocess.check_output(["xwininfo", "-root", "-tree"], text=True, stderr=subprocess.STDOUT)
        except subprocess.CalledProcessError as error:
            raise RuntimeError(f"Cannot query X11 windows: {error.output.strip()}") from error
        for line in tree.splitlines():
            match = re.match(r'\s*(0x[0-9a-fA-F]+) "' + re.escape(title) + r'"', line)
            if not match:
                continue
            window = match.group(1)
            owner = subprocess.check_output(["xprop", "-id", window, "_NET_WM_PID"], text=True)
            owner_match = re.search(r"=\s*(\d+)\s*$", owner)
            owner_pid = int(owner_match.group(1)) if owner_match else window_client_pid(int(window, 16))
            if owner_pid == process.pid:
                details = subprocess.check_output(["xwininfo", "-id", window], text=True)
                if "Map State: IsViewable" in details:
                    return int(window, 16)
        time.sleep(0.05)
    raise RuntimeError(f"No X11 window titled {title!r} owned by PID {process.pid}. See application.log")


def connect_control(process, name, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        check_alive(process)
        control = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
        try:
            control.connect("\0" + name)
            control.setblocking(False)
            return control
        except (ConnectionRefusedError, FileNotFoundError):
            control.close()
            time.sleep(0.05)
    raise RuntimeError("MangoHud control socket did not appear")


@contextlib.contextmanager
def launch_app(app, binary, output, arguments):
    output.mkdir()
    if any(character in str(output) for character in ",\n"):
        raise ValueError("MangoHud output paths cannot contain commas or newlines")
    control_name = "gui-capture-" + uuid.uuid4().hex
    environment = os.environ.copy()
    for variable in ("WAYLAND_DISPLAY", "WAYLAND_SOCKET", "MANGOHUD_CONFIGFILE"):
        environment.pop(variable, None)
    environment.update(app.get("env", {}))
    environment["WINIT_UNIX_BACKEND"] = "x11"
    environment["MANGOHUD_CONFIG"] = (
        f"fps,frametime,frame_timing,cpu_stats=0,gpu_stats=0,permit_upload=0,"
        f"fps_limit={arguments.fps_limit},log_interval=0,control={control_name},output_folder={output}"
    )
    if arguments.no_vsync:
        environment["MANGOHUD_CONFIG"] += ",gl_vsync=0,vsync=1"
    with (output / "application.log").open("w") as log:
        process = subprocess.Popen(["mangohud", "--dlsym", str(binary)], env=environment,
                                   stdout=log, stderr=subprocess.STDOUT, cwd=ROOT)
        try:
            yield process, control_name
        finally:
            if process.poll() is None:
                process.terminate()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait()


def capture(app, binary, output, arguments):
    if getattr(arguments, "benchmark", "scroll") == "dashboard":
        return capture_dashboard(app, binary, output, arguments)
    return capture_scroll(app, binary, output, arguments)


def capture_scroll(app, binary, output, arguments):
    with launch_app(app, binary, output, arguments) as (process, control_name):
        with contextlib.ExitStack() as stack:
            window = wait_for_window(process, app["title"], arguments.startup_timeout)
            with WheelInput(window) as wheel:
                control = stack.enter_context(connect_control(process, control_name, arguments.startup_timeout))
                messages = bytearray()
                trace = None
                phase = None

                def tick():
                    nonlocal trace
                    check_alive(process)
                    while True:
                        try:
                            data = control.recv(4096)
                        except BlockingIOError:
                            break
                        if not data:
                            raise RuntimeError("MangoHud control socket disconnected")
                        messages.extend(data)
                    if trace is None:
                        files = [path for path in output.glob("*.csv") if not path.name.endswith("_summary.csv")]
                        if len(files) > 1:
                            raise ValueError("Multiple raw MangoHud CSVs in one capture")
                        if files:
                            trace = stack.enter_context(FrameLog(files[0], validate_reported_frametime=app.get("validate_reported_frametime", True)))
                    if trace:
                        trace.poll()
                    if phase.end_events is None:
                        return phase.update(trace.frames if trace else [], time.monotonic(), wheel.events_sent)
                    return True

                time.sleep(0.25)
                control.sendall(b":logging=1;")
                started = time.monotonic()
                phase = CaptureWindow(arguments.duration, arguments.warmup, arguments.startup_timeout, started)
                try:
                    wheel.scroll(arguments.warmup + arguments.duration + 2 * arguments.startup_timeout + 1,
                                 arguments.rate, tick)
                    if phase.end_events is None:
                        raise RuntimeError("Scrolling ended before a complete frame window was recorded")
                    input_duration = time.monotonic() - started
                except BaseException:
                    with contextlib.suppress(OSError):
                        control.sendall(b":logging=0;")
                    raise
                control.sendall(b":logging=0;")
                deadline = time.monotonic() + 5
                while b"LoggingFinished" not in messages:
                    if time.monotonic() > deadline:
                        raise RuntimeError("MangoHud did not acknowledge the end of logging")
                    wheel.scroll(0.05, arguments.rate, tick)
                trace.finish()
                metrics = summarize(trace.frames, phase.start_ns, arguments.duration)
                csv_path = trace.path
                size = wheel.size
                total_notches = wheel.events_sent
                mango_info = messages.decode(errors="replace")
            result = {
                "app": app["id"], "status": "ok", "requested_renderer": app["renderer"],
                "dependencies": tomllib.loads(app["manifest"].read_text())["dependencies"],
                "display_backend": "x11", "client_pixels": list(size),
                "fps_limit": arguments.fps_limit,
                "vsync": "disabled-requested" if arguments.no_vsync else "application-default",
                "wheel_rate_hz": arguments.rate,
                "wheel_notches_while_measuring": phase.end_events - phase.start_events,
                "observed_input_hz": (phase.end_events - phase.start_events) / (phase.end_wall - phase.start_wall),
                "total_wheel_notches": total_notches, "input_duration_s": input_duration,
                "readiness_delay_s": phase.start_wall - started,
                "warmup_s": arguments.warmup, "csv": str(csv_path),
                "mangohud_control": mango_info, "metrics": metrics,
                "timing_validation": {"reported_frametime_crosschecked": trace.validate_reported_frametime,
                                      "reported_timing_mismatches": trace.reported_timing_mismatches},
            }
            return result


def capture_dashboard(app, binary, output, arguments):
    with launch_app(app, binary, output, arguments) as (process, control_name):
        with contextlib.ExitStack() as stack:
            window = wait_for_window(process, app["title"], arguments.startup_timeout)
            with dashboard.DashboardInput(window) as input:
                control = stack.enter_context(connect_control(process, control_name, arguments.startup_timeout))
                messages = bytearray()
                trace = None

                def poll():
                    nonlocal trace
                    check_alive(process)
                    while True:
                        try:
                            data = control.recv(4096)
                        except BlockingIOError:
                            break
                        if not data:
                            raise RuntimeError("MangoHud control socket disconnected")
                        messages.extend(data)
                    if trace is None:
                        files = [path for path in output.glob("*.csv") if not path.name.endswith("_summary.csv")]
                        if len(files) > 1:
                            raise ValueError("Multiple raw MangoHud CSVs in dashboard capture")
                        if files:
                            trace = stack.enter_context(FrameLog(
                                files[0], validate_reported_frametime=app.get("validate_reported_frametime", True)))
                    if trace:
                        trace.poll()

                input.perform("focus-input")
                input.perform("select-text")
                input.perform("backspace")
                time.sleep(arguments.warmup)
                control.sendall(b":logging=1;")
                try:
                    deadline = time.monotonic() + arguments.startup_timeout
                    while trace is None or len(trace.frames) < 2:
                        if time.monotonic() >= deadline:
                            raise RuntimeError("No dashboard frame anchor before the workload")
                        input.perform("type-a")
                        input.perform("backspace")
                        time.sleep(0.005)
                        poll()
                    start_ns = trace.frames[-1].elapsed_ns
                    workload = dashboard.exercise(input, poll, arguments.rate)
                    end_ns = start_ns + round(workload["duration_s"] * 1e9)
                    deadline = time.monotonic() + arguments.startup_timeout
                    while trace.frames[-1].elapsed_ns < end_ns:
                        if time.monotonic() >= deadline:
                            raise RuntimeError("Dashboard capture did not bracket the full eight-second workload")
                        input.click(496, 90)
                        input.wheel(5)
                        time.sleep(0.01)
                        poll()
                finally:
                    with contextlib.suppress(OSError):
                        control.sendall(b":logging=0;")
                deadline = time.monotonic() + arguments.startup_timeout
                while b"LoggingFinished" not in messages:
                    if time.monotonic() >= deadline:
                        raise RuntimeError("MangoHud did not acknowledge the end of logging")
                    input.click(496, 90)
                    input.wheel(5)
                    time.sleep(0.05)
                    poll()
                application_log = (output / "application.log").read_text(errors="replace")
                if "UnsupportedCustomElement" in application_log or "panicked at" in application_log:
                    raise RuntimeError("Application reported a runtime UI failure, see application.log")
                trace.finish()
                metrics = dashboard.summarize(trace.frames, start_ns, workload["duration_s"])
                validation = {"reported_frametime_crosschecked": trace.validate_reported_frametime,
                              "reported_timing_mismatches": trace.reported_timing_mismatches}
                return {
                    "app": app["id"], "benchmark": "dashboard", "status": "ok",
                    "requested_renderer": app["renderer"], "display_backend": "x11",
                    "dependencies": tomllib.loads(app["manifest"].read_text())["dependencies"],
                    "client_pixels": list(input.size), "fps_limit": arguments.fps_limit,
                    "vsync": "disabled-requested" if arguments.no_vsync else "application-default",
                    "warmup_s": arguments.warmup, "input_rate_hz": arguments.rate,
                    "phase_duration_s": dashboard.PHASE_DURATION,
                    "workload_duration_s": workload["duration_s"],
                    "actual_workload_duration_s": workload["actual_duration_s"],
                    "phases": workload["phases"], "actions": workload["actions"],
                    "csv": str(trace.path), "metrics": metrics,
                    "input_scope": "Fixed XTest actions sent to the window. Widget effects are not automatically verified.",
                    "mangohud_control": messages.decode(errors="replace"), "timing_validation": validation,
                }


def run_benchmarks(arguments, apps):
    requested = [name for argument in arguments.apps for name in argument.split(",")]
    if not requested or requested == ["all"]:
        requested = list(apps)
    if not requested or any(name not in apps for name in requested):
        raise ValueError("Unknown or missing app. Use the list command for available apps")
    requested = list(dict.fromkeys(requested))
    for command in ("cargo", "mangohud", "xwininfo", "xprop"):
        if not shutil.which(command):
            raise ValueError(f"Required command not found: {command}")
    if not os.environ.get("DISPLAY"):
        raise ValueError("An X11 or XWayland DISPLAY is required")
    benchmark = getattr(arguments, "benchmark", "scroll")
    run = new_run(["run", *sys.argv[2:]], benchmark)
    output = arguments.output.resolve() / run["id"]
    output.mkdir(parents=True)
    run["output_directory"] = str(output)
    binaries = {}
    for name in requested:
        try:
            binaries[name] = build(apps[name], output, arguments.no_build, benchmark)
        except (OSError, ValueError, subprocess.SubprocessError) as error:
            run["results"].append({"app": name, "status": "failed", "stage": "build", "error": str(error)})
    for name in requested:
        if name not in binaries:
            continue
        for repetition in range(1, arguments.repetitions + 1):
            print(f"Capturing {name} ({repetition}/{arguments.repetitions})...", file=sys.stderr)
            destination = output / f"{name}-{repetition}"
            try:
                result = capture(apps[name], binaries[name], destination, arguments)
            except (OSError, ValueError, RuntimeError, subprocess.SubprocessError) as error:
                result = {"app": name, "status": "failed", "stage": "capture", "error": str(error)}
            run["results"].append({**result, "repetition": repetition})
    (output / "run.json").write_text(json.dumps(run, indent=2, allow_nan=False) + "\n")
    finish(run, arguments)
    return int(any(result["status"] != "ok" for result in run["results"]))


def positive_integer(value):
    number = int(value)
    if number <= 0:
        raise argparse.ArgumentTypeError("must be positive")
    return number


def nonnegative_integer(value):
    number = int(value)
    if number < 0:
        raise argparse.ArgumentTypeError("must be nonnegative")
    return number


def history_options(parser):
    parser.add_argument("--save", action="store_true", help="Append this run to history, otherwise leave history untouched")
    parser.add_argument("--history", type=Path)


def main():
    parser = argparse.ArgumentParser(description="External MangoHud scroll and dashboard benchmarks, no application instrumentation.")
    commands = parser.add_subparsers(dest="command", required=True)
    commands.add_parser("list", help="List standalone apps")
    run = commands.add_parser("run", help="Build and capture apps sequentially")
    run.add_argument("apps", nargs="*", help="App IDs, comma-separated IDs, or all (default)")
    run.add_argument("--benchmark", choices=("scroll", "dashboard"), default="scroll")
    run.add_argument("--duration", type=positive_number, default=15)
    run.add_argument("--warmup", type=positive_number, default=2)
    run.add_argument("--rate", type=positive_number,
                     help="Input events per second, dashboard defaults to 240, scroll follows the FPS cap")
    run.add_argument("--fps-limit", type=nonnegative_integer,
                     help="MangoHud FPS limit, defaults to 0 for dashboard and 144 for scroll")
    synchronization = run.add_mutually_exclusive_group()
    synchronization.add_argument("--no-vsync", dest="no_vsync", action="store_true", default=None,
                                 help="Request disabled synchronization, the dashboard default")
    synchronization.add_argument("--vsync", dest="no_vsync", action="store_false",
                                 help="Keep application-default synchronization")
    run.add_argument("--repetitions", type=positive_integer, default=1)
    run.add_argument("--startup-timeout", type=positive_number, default=30)
    run.add_argument("--output", type=Path, default=ROOT / "results" / "apps")
    run.add_argument("--no-build", action="store_true", help="Use existing release binaries")
    history_options(run)
    parse = commands.add_parser("parse", help="Parse an existing per-frame MangoHud CSV")
    parse.add_argument("csv", type=Path)
    parse.add_argument("--benchmark", choices=("scroll", "dashboard"), default="scroll")
    parse.add_argument("--app", required=True)
    parse.add_argument("--duration", type=positive_number, help="Scroll measurement duration, fully bracketed by CSV frames")
    parse.add_argument("--start", type=float, help="Earliest measurement start in CSV elapsed seconds, defaults to first frame")
    history_options(parse)
    arguments = parser.parse_args()
    if arguments.command != "list" and arguments.history is None:
        filename = "dashboard-history.json" if arguments.benchmark == "dashboard" else "history.json"
        arguments.history = DIRECTORY / filename
    if arguments.command == "parse":
        if arguments.benchmark == "scroll" and arguments.duration is None:
            parser.error("scroll parsing requires --duration")
        if arguments.benchmark == "dashboard" and (arguments.duration is not None or arguments.start is not None):
            parser.error("dashboard parsing uses the complete capture, without --duration or --start")
    try:
        apps = load_apps()
        if arguments.command == "list":
            for app in apps.values():
                print(f"{app['id']:20} {app['renderer']:8} {app['title']}")
            return 0
        if arguments.command == "run":
            if arguments.fps_limit is None:
                arguments.fps_limit = 0 if arguments.benchmark == "dashboard" else 144
            if arguments.no_vsync is None:
                arguments.no_vsync = arguments.benchmark == "dashboard"
            if arguments.rate is None:
                arguments.rate = 240 if arguments.benchmark == "dashboard" else arguments.fps_limit or 144
            return run_benchmarks(arguments, apps)
        if arguments.app not in apps:
            raise ValueError(f"Unknown app: {arguments.app}")
        run = new_run(["parse", *sys.argv[2:]], arguments.benchmark)
        if arguments.benchmark == "dashboard":
            with FrameLog(arguments.csv) as trace:
                trace.finish()
                metrics = dashboard.summarize(trace.frames)
        else:
            metrics = parse_csv(arguments.csv, arguments.duration, arguments.start)
        run["results"].append({
            "app": arguments.app, "status": "ok", "source": "imported-csv",
            "csv": str(arguments.csv.resolve()),
            "capture_configuration": "unknown, must be a log_interval=0 per-frame capture",
            "metrics": metrics,
        })
        finish(run, arguments)
        return 0
    except (OSError, ValueError, KeyError, StopIteration, subprocess.SubprocessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("Interrupted, history not saved", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main())
