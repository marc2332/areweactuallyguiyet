#!/usr/bin/env python3
"""Publish MangoHud captures using the website's existing collection format."""
import argparse
import collections
import datetime
import fcntl
import json
import math
from pathlib import Path
import statistics
import subprocess
import sys
import tempfile

import bench

BOUNDARY = "mangohud-present-observation"
DASHBOARD_MEASUREMENTS = ("mangohud-dashboard-v1", "mangohud-dashboard-v2")


def dependency_versions(dependencies):
    versions = []
    for name, specification in dependencies.items():
        if isinstance(specification, str):
            version = specification
        else:
            version = specification.get("rev", specification.get("version", "path " + specification.get("path", "unreported")))
            if specification.get("features"):
                version += " (" + ", ".join(specification["features"]) + ")"
        versions.append(f"{name} {version}")
    return " · ".join(versions)


def website_run(source, apps, libraries):
    if source.get("measurement") in DASHBOARD_MEASUREMENTS:
        return dashboard_website_run(source, apps, libraries)
    if source.get("measurement") != bench.MEASUREMENT or not source.get("results"):
        raise ValueError("Expected a supported MangoHud run with results")
    datetime.datetime.fromisoformat(source["timestamp"])
    names = {library["id"]: library["name"] for library in libraries}
    samples = collections.defaultdict(list)
    for result in source["results"]:
        if result.get("app") not in apps or result.get("status") not in ("ok", "failed"):
            raise ValueError("Unknown application or result status")
        samples[result["app"]].append(result)
    metadata = {
        "buildProfile": "release", "measurement": bench.MEASUREMENT,
        "responsivenessVersion": 3,
        "responsivenessScope": "MangoHud present observations, not confirmed displayed frames or present-return timing",
        "sourceRun": source["id"], "frameworkLabels": {}, "frameworkBaseIds": {}, "frameworkVersions": {},
    }
    results = []
    configurations = set()
    for identifier, captures in samples.items():
        base = next((name for name in sorted(names, key=len, reverse=True)
                     if identifier == name or identifier.startswith(name + "-")), None)
        if base is None:
            raise ValueError(f"No website library for {identifier}")
        metadata["frameworkBaseIds"][identifier] = base
        label = apps[identifier]["title"].removesuffix(" Scroll 2000")
        if identifier != base and label == names[base]:
            label += " (" + identifier[len(base) + 1:].replace("-", " ").title() + ")"
        metadata["frameworkLabels"][identifier] = names[base] if identifier == base else label
        repetitions = []
        failures = []
        indices = set()
        for capture in captures:
            index = capture.get("repetition", 1)
            if type(index) is not int or index < 1 or index in indices:
                raise ValueError("Invalid or duplicate repetition")
            indices.add(index)
            if capture["status"] != "ok":
                failures.append(capture.get("error", "Capture failed"))
                repetitions.append({"index": index - 1, "metrics": {}, "status": "failed", "reason": failures[-1]})
                continue
            timing = capture["metrics"]
            duration = timing["measurement_duration_s"]
            fps = timing["fps"]
            if not all(isinstance(value, (float, int)) and not isinstance(value, bool) and math.isfinite(value) for value in (duration, fps)) or duration <= 0 or fps < 0 or not math.isclose(fps, timing["frames"] / duration, rel_tol=1e-9, abs_tol=1e-9):
                raise ValueError("Invalid capture duration or frame count")
            configurations.add((duration, capture["warmup_s"], capture["fps_limit"], capture["vsync"], capture["wheel_rate_hz"]))
            metrics = {"renderer_fps": {"value": fps, "unit": "frames/s"},
                       "renderer_frames": {"value": timing["frames"], "unit": "frames"},
                       "frame_gap_max_ms": {"value": timing["max_frame_gap_ms"], "unit": "ms"}}
            for percentile in ("p95", "p99"):
                if timing.get("frame_interval_ms"):
                    metrics[f"frame_gap_{percentile}_ms"] = {"value": timing["frame_interval_ms"][percentile], "unit": "ms"}
            repetitions.append({"index": index - 1, "metrics": metrics, "status": "ok", "events": [{
                "event": "mangohud-capture", "trace": {"boundary": BOUNDARY, "csv": capture["csv"],
                    "start_elapsed_ns": timing["measurement_start_elapsed_ns"], "end_elapsed_ns": timing["measurement_end_elapsed_ns"],
                    "timing_validation": capture.get("timing_validation", {"reported_frametime_crosschecked": True})},
            }]})
            version = dependency_versions(capture["dependencies"])
            previous = metadata["frameworkVersions"].setdefault(identifier, version)
            if previous != version:
                raise ValueError("Dependencies changed between repetitions")
        successful = [repetition for repetition in repetitions if repetition["status"] == "ok"]
        metrics = {}
        for metric in {metric for repetition in successful for metric in repetition["metrics"]}:
            values = [repetition["metrics"][metric] for repetition in successful if metric in repetition["metrics"]]
            metrics[metric] = {"value": statistics.median(value["value"] for value in values), "unit": values[0]["unit"]}
        results.append({"framework": identifier, "scenario": "scroll", "scenarioVersion": 2,
                        "status": "ok" if successful else "failed", "reason": " · ".join(failures) or None,
                        "metrics": metrics, "repetitions": repetitions})
    if len(configurations) != 1:
        raise ValueError("Expected successful captures with one consistent configuration")
    duration, warmup, fps_limit, vsync, rate = configurations.pop()
    metadata.update({"durationSeconds": duration, "warmupSeconds": warmup, "fpsLimit": fps_limit,
                     "vsync": vsync, "wheelRateHz": rate, "repetitions": max(map(len, samples.values()))})
    return {"id": source["id"], "collectedAt": source["timestamp"],
            "machine": {"id": "unreported", "label": "External MangoHud capture", "os": source.get("system", {}).get("os"), "cpu": None, "gpu": None},
            "results": results, "metadata": metadata}


def dashboard_website_run(source, apps, libraries):
    if not source.get("results"):
        raise ValueError("Expected a MangoHud dashboard run with results")
    datetime.datetime.fromisoformat(source["timestamp"])
    names = {library["id"]: library["name"] for library in libraries}
    samples = collections.defaultdict(list)
    configurations = set()
    for capture in source["results"]:
        if capture.get("app") not in apps or capture.get("status") not in ("ok", "failed"):
            raise ValueError("Unknown application or result status")
        samples[capture["app"]].append(capture)

    results = []
    metadata = {
        "buildProfile": "release", "measurement": source["measurement"],
        "sourceRun": source["id"], "frameworkLabels": {}, "frameworkBaseIds": {}, "frameworkVersions": {},
    }
    for identifier, captures in samples.items():
        base = next((name for name in sorted(names, key=len, reverse=True)
                     if identifier == name or identifier.startswith(name + "-")), None)
        if base is None:
            raise ValueError(f"No website library for {identifier}")
        metadata["frameworkBaseIds"][identifier] = base
        label = apps[identifier]["title"].removesuffix(" Scroll 2000")
        if identifier != base and label == names[base]:
            label += " (" + identifier[len(base) + 1:].replace("-", " ").title() + ")"
        metadata["frameworkLabels"][identifier] = names[base] if identifier == base else label
        repetitions = []
        failures = []
        indices = set()
        for capture in captures:
            index = capture.get("repetition", 1)
            if type(index) is not int or index < 1 or index in indices:
                raise ValueError("Invalid or duplicate repetition")
            indices.add(index)
            if capture["status"] != "ok":
                failures.append(capture.get("error", "Capture failed"))
                repetitions.append({"index": index - 1, "metrics": {}, "status": "failed", "reason": failures[-1]})
                continue
            metrics = capture["metrics"]
            summary = metrics["present_interval_ms"]
            intervals = [sample["present_interval_ms"] for sample in metrics["per_frame"]]
            values = [*(summary.values() if summary else []), *intervals]
            if not all(isinstance(value, (int, float)) and not isinstance(value, bool) and math.isfinite(value) and value >= 0 for value in values):
                raise ValueError("Invalid dashboard present intervals")
            if summary:
                if not intervals or not math.isclose(max(intervals), summary["max"], rel_tol=1e-9, abs_tol=1e-9):
                    raise ValueError("Dashboard maximum does not match per-frame intervals")
            elif intervals or "worst_presentation_gap_ms" not in metrics:
                raise ValueError("Missing dashboard interval summary")
            configurations.add((capture["warmup_s"], capture.get("action_delay_s"), capture["fps_limit"],
                                capture["vsync"], capture.get("phase_duration_s"), capture.get("input_rate_hz")))
            repetition_metrics = {
                f"dashboard_present_{name}_ms": {"value": summary[name], "unit": "ms"}
                for name in ("median", "p95", "max")
            } if summary else {}
            if "worst_presentation_gap_ms" in metrics:
                worst = metrics["worst_presentation_gap_ms"]
                trailing = metrics["trailing_gap_ms"]
                duration = metrics["measurement_duration_s"]
                if not all(isinstance(value, (int, float)) and not isinstance(value, bool)
                           and math.isfinite(value) and value >= 0 for value in (worst, trailing, duration)):
                    raise ValueError("Invalid dashboard workload window")
                if (duration != 8 or metrics["measurement_end_elapsed_ns"] - metrics["measurement_start_elapsed_ns"] != 8_000_000_000
                        or worst > 8000 or trailing > 8000
                        or not math.isclose(sum(intervals) + trailing, 8000, rel_tol=1e-9, abs_tol=1e-6)
                        or not math.isclose(worst, max([trailing, *intervals]), rel_tol=1e-9, abs_tol=1e-9)):
                    raise ValueError("Dashboard worst gap does not match the full eight-second window")
                repetition_metrics["dashboard_worst_gap_ms"] = {"value": worst, "unit": "ms"}
            repetitions.append({"index": index - 1, "metrics": repetition_metrics, "status": "ok", "events": [{
                "event": "dashboard-present-intervals", "trace": {
                    "boundary": BOUNDARY, "csv": capture["csv"], "timing_scope": metrics["timing_scope"],
                    "per_frame": metrics["per_frame"], "actions": capture["actions"],
                    "phases": capture.get("phases", []),
                    "measurement_window": {key: metrics[key] for key in (
                        "measurement_start_elapsed_ns", "measurement_end_elapsed_ns",
                        "measurement_duration_s", "trailing_gap_ms") if key in metrics},
                    "timing_validation": capture.get("timing_validation", {}),
                },
            }]})
            version = dependency_versions(capture["dependencies"])
            previous = metadata["frameworkVersions"].setdefault(identifier, version)
            if previous != version:
                raise ValueError("Dependencies changed between repetitions")
        successful = [repetition for repetition in repetitions if repetition["status"] == "ok"]
        aggregate = {}
        for metric in {metric for repetition in successful for metric in repetition["metrics"]}:
            values = [repetition["metrics"][metric] for repetition in successful if metric in repetition["metrics"]]
            combine = max if metric == "dashboard_worst_gap_ms" else statistics.median
            aggregate[metric] = {"value": combine(value["value"] for value in values), "unit": "ms"}
        results.append({"framework": identifier, "scenario": "dashboard",
                        "scenarioVersion": 2 if source["measurement"] == "mangohud-dashboard-v2" else 1,
                        "status": "ok" if successful else "failed", "reason": " · ".join(failures) or None,
                        "metrics": aggregate, "repetitions": repetitions})
    if len(configurations) > 1:
        raise ValueError("Expected successful dashboard captures with one consistent configuration")
    metadata["repetitions"] = max(map(len, samples.values()))
    if configurations:
        warmup, action_delay, fps_limit, vsync, phase_duration, rate = configurations.pop()
        metadata.update({"warmupSeconds": warmup, "fpsLimit": fps_limit, "vsync": vsync})
        if phase_duration is not None:
            metadata.update({"phaseDurationSeconds": phase_duration, "durationSeconds": phase_duration * 4,
                             "inputRateHz": rate})
        else:
            metadata["actionDelaySeconds"] = action_delay
    return {"id": source["id"], "collectedAt": source["timestamp"],
            "machine": {"id": "unreported", "label": "External MangoHud capture", "os": source.get("system", {}).get("os"), "cpu": None, "gpu": None},
            "results": results, "metadata": metadata}


def merge_latest(history, run):
    if not history.get("runs"):
        raise ValueError("Cannot merge dashboard results without an existing website run")
    latest = max(history["runs"], key=lambda previous: datetime.datetime.fromisoformat(previous["collectedAt"]))
    dashboard = [result for result in run["results"] if result["scenario"] == "dashboard"]
    keys = {(result["framework"], result["scenario"]) for result in dashboard}
    latest["results"] = [result for result in latest["results"] if (result["framework"], result["scenario"]) not in keys] + dashboard
    collections = latest["metadata"].setdefault("scenarioCollections", {})
    previous = collections.get("dashboard", {})
    metadata = {**previous, **run["metadata"]}
    for key in ("frameworkLabels", "frameworkBaseIds", "frameworkVersions"):
        metadata[key] = {**previous.get(key, {}), **run["metadata"].get(key, {})}
    collections["dashboard"] = metadata
    return history


def save(path, run, latest=False):
    path = path.resolve()
    with path.with_name(path.name + ".lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        history = json.loads(path.read_text())
        if history.get("schemaVersion") != 1 or not isinstance(history.get("runs"), list):
            raise ValueError("Expected the existing website history format")
        if latest:
            if run["metadata"].get("measurement") not in DASHBOARD_MEASUREMENTS:
                raise ValueError("--latest is only supported for dashboard captures")
            merge_latest(history, run)
        else:
            if any(previous["id"] == run["id"] for previous in history["runs"]):
                raise ValueError("This capture has already been published")
            history["runs"].append(run)
        content = json.dumps(history, indent=2, allow_nan=False, ensure_ascii=False) + "\n"
        subprocess.run(["node", "--input-type=module", "-e",
                        "import {validateHistory} from './site/src/lib/history.mjs'; let input=''; for await (const chunk of process.stdin) input+=chunk; validateHistory(JSON.parse(input));"],
                       cwd=bench.ROOT, input=content, text=True, check=True)
        temporary = None
        try:
            with tempfile.NamedTemporaryFile(mode="w", dir=path.parent, delete=False) as destination:
                temporary = Path(destination.name)
                destination.write(content)
            temporary.chmod(path.stat().st_mode & 0o777)
            temporary.replace(path)
        finally:
            if temporary:
                temporary.unlink(missing_ok=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run", type=Path, help="A completed bench.py run.json artifact")
    parser.add_argument("--history", type=Path, default=bench.ROOT / "site/data/history.json")
    parser.add_argument("--save", action="store_true", help="Publish to website history, otherwise print only")
    parser.add_argument("--latest", action="store_true", help="Merge dashboard results into the latest website run")
    arguments = parser.parse_args()
    if arguments.latest and not arguments.save:
        parser.error("--latest requires --save")
    try:
        libraries = json.loads((bench.ROOT / "site/data/libraries.json").read_text())["libraries"]
        run = website_run(json.loads(arguments.run.read_text()), bench.load_apps(), libraries)
        if arguments.save:
            save(arguments.history, run, latest=arguments.latest)
            destination_id = run["id"] if not arguments.latest else "latest run"
            print(f"Published {destination_id} to {arguments.history}")
        else:
            print(json.dumps(run, indent=2, allow_nan=False))
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
