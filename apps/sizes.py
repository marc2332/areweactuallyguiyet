#!/usr/bin/env python3
"""Collect production executable file sizes without launching the GUI apps."""
import argparse
import datetime
import json
from pathlib import Path
import platform
import subprocess
import sys
import tempfile
import tomllib

import bench


def collect(apps, output, skip_build):
    results = []
    for app in apps.values():
        binary = bench.build(app, output, skip_build, profile="production")
        manifest = tomllib.loads(app["manifest"].read_text())
        name = app["title"].removesuffix(" Scroll 2000")
        base = next((identifier for identifier in sorted(apps, key=len, reverse=True)
                     if app["id"].startswith(identifier + "-")), None)
        if base and app["title"] == apps[base]["title"]:
            name += " (" + app["id"][len(base) + 1:].replace("-", " ").title() + ")"
        results.append({
            "framework": app["id"],
            "name": name,
            "binary": binary.name,
            "bytes": binary.stat().st_size,
            "dependencies": manifest["dependencies"],
        })
    return {
        "schemaVersion": 1,
        "collectedAt": datetime.datetime.now(datetime.timezone.utc).isoformat(),
        "platform": {"os": platform.system(), "architecture": platform.machine()},
        "profile": "production",
        "method": "Executable file size as built, no additional stripping or compression. Shared libraries and external assets excluded.",
        "results": results,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--no-build", action="store_true", help="Measure existing production binaries without rebuilding")
    parser.add_argument("--output", type=Path, default=bench.ROOT / "site/data/binary-sizes-snapshot.json")
    arguments = parser.parse_args()
    try:
        with tempfile.TemporaryDirectory(prefix="gui-binary-sizes-") as directory:
            snapshot = collect(bench.load_apps(), Path(directory), arguments.no_build)
        arguments.output.parent.mkdir(parents=True, exist_ok=True)
        arguments.output.write_text(json.dumps(snapshot, indent=2, allow_nan=False) + "\n")
        print(f"Saved {len(snapshot['results'])} executable sizes to {arguments.output}")
        return 0
    except (OSError, ValueError, StopIteration, subprocess.SubprocessError) as error:
        print(f"Error: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
