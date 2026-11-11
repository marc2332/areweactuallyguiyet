"""Collect production executable file sizes without launching the GUI apps."""
import datetime
import platform
import tomllib

from benchmark import runner


def collect(apps, output, skip_build):
    results = []
    for app in apps.values():
        binary = runner.build(app, output, skip_build, profile="production")
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
