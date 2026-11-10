import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bench
import publish


def source_run():
    return {
        "id": "fixture", "timestamp": "2026-11-10T20:00:00+00:00", "measurement": bench.MEASUREMENT,
        "system": {"os": "Linux"}, "results": [{
            "app": "whisker", "status": "ok", "repetition": 1, "warmup_s": 1,
            "fps_limit": 144, "vsync": "application-default", "wheel_rate_hz": 144,
            "csv": "/capture.csv", "dependencies": {"whisker": {"rev": "a" * 40}},
            "metrics": {"measurement_duration_s": 3, "fps": 10, "frames": 30,
                        "max_frame_gap_ms": 100, "frame_interval_ms": {"p95": 100, "p99": 100},
                        "measurement_start_elapsed_ns": 3000000000, "measurement_end_elapsed_ns": 6000000000},
        }],
    }


class PublishTests(unittest.TestCase):
    def setUp(self):
        self.apps = bench.load_apps()
        self.libraries = [{"id": "whisker", "name": "Whisker"}, {"id": "dioxus-native", "name": "Dioxus Native"}]
        self.source = source_run()

    def convert(self):
        return publish.website_run(self.source, self.apps, self.libraries)

    def test_uses_existing_history_structure_and_honest_boundary(self):
        run = self.convert()
        self.assertEqual(set(run), {"id", "collectedAt", "machine", "metadata", "results"})
        result = run["results"][0]
        self.assertEqual(set(result), {"framework", "scenario", "scenarioVersion", "status", "reason", "metrics", "repetitions"})
        self.assertEqual(result["metrics"]["renderer_fps"], {"value": 10, "unit": "frames/s"})
        event = result["repetitions"][0]["events"][0]
        self.assertEqual(event["event"], "mangohud-capture")
        self.assertEqual(event["trace"]["boundary"], "mangohud-present-observation")

    def test_aggregates_repetitions_and_preserves_each_sample(self):
        second = copy.deepcopy(self.source["results"][0])
        second["repetition"] = 2
        second["metrics"].update(fps=20, frames=60)
        self.source["results"].append(second)
        result = self.convert()["results"][0]
        self.assertEqual(result["metrics"]["renderer_fps"]["value"], 15)
        self.assertEqual(len(result["repetitions"]), 2)

    def test_variant_labels_and_failed_samples_are_retained(self):
        self.source["results"].append({"app": "dioxus-native-vello-hybrid", "status": "failed", "repetition": 1, "error": "No capture"})
        run = self.convert()
        self.assertEqual(run["metadata"]["frameworkBaseIds"]["dioxus-native-vello-hybrid"], "dioxus-native")
        self.assertEqual(run["metadata"]["frameworkLabels"]["dioxus-native-vello-hybrid"], "Dioxus Native Vello Hybrid")
        self.assertEqual(run["results"][1]["status"], "failed")
        self.assertEqual(run["results"][1]["metrics"], {})

    def test_incompatible_measurements_duplicate_samples_and_mixed_settings_fail(self):
        for change in ("identity", "duplicate", "settings", "clock"):
            with self.subTest(change=change):
                self.source = source_run()
                if change == "identity":
                    self.source["measurement"] = "other-measurement"
                elif change == "clock":
                    self.source["results"][0]["metrics"]["fps"] = 100
                else:
                    second = copy.deepcopy(self.source["results"][0])
                    if change == "settings":
                        second.update(repetition=2, fps_limit=60)
                    self.source["results"].append(second)
                with self.assertRaises(ValueError):
                    self.convert()

    def test_dry_run_does_not_touch_destination(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            source = root / "run.json"
            source.write_text(json.dumps(self.source))
            destination = root / "absent" / "history.json"
            result = subprocess.run([sys.executable, str(bench.DIRECTORY / "publish.py"), str(source), "--history", str(destination)], capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertFalse(destination.parent.exists())

    def test_explicit_save_preserves_prior_collections_and_rejects_duplicates(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "history.json"
            previous = self.convert()
            previous["id"] = "previous"
            path.write_text(json.dumps({"schemaVersion": 1, "runs": [previous]}))
            run = self.convert()
            publish.save(path, run)
            history = json.loads(path.read_text())
            self.assertEqual(history["runs"], [previous, run])
            with self.assertRaisesRegex(ValueError, "already been published"):
                publish.save(path, run)
            self.assertEqual(json.loads(path.read_text()), history)


if __name__ == "__main__":
    unittest.main()
