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


def website_run(source, apps, libraries):
    if source.get("measurement") != bench.MEASUREMENT or not source.get("results"):
        raise ValueError("Expected a mangohud-scroll-v2 run with results")
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
            versions = []
            for dependency, specification in capture["dependencies"].items():
                if isinstance(specification, str):
                    version = specification
                else:
                    version = specification.get("rev", specification.get("version", "path " + specification.get("path", "unreported")))
                    if specification.get("features"):
                        version += " (" + ", ".join(specification["features"]) + ")"
                versions.append(f"{dependency} {version}")
            version = " · ".join(versions)
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


def save(path, run):
    path = path.resolve()
    with path.with_name(path.name + ".lock").open("a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        history = json.loads(path.read_text())
        if history.get("schemaVersion") != 1 or not isinstance(history.get("runs"), list):
            raise ValueError("Expected the existing website history format")
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
    parser.add_argument("--save", action="store_true", help="Append to website history, otherwise print only")
    arguments = parser.parse_args()
    try:
        libraries = json.loads((bench.ROOT / "site/data/libraries.json").read_text())["libraries"]
        run = website_run(json.loads(arguments.run.read_text()), bench.load_apps(), libraries)
        if arguments.save:
            save(arguments.history, run)
            print(f"Published {run['id']} to {arguments.history}")
        else:
            print(json.dumps(run, indent=2, allow_nan=False))
        return 0
    except (OSError, ValueError, KeyError, subprocess.SubprocessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
