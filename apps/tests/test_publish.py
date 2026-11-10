import copy
import json
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bench
import dashboard
from frames import Frame
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


def dashboard_source():
    return {
        "id": "dashboard-fixture", "timestamp": "2026-11-10T20:00:00+00:00",
        "measurement": "mangohud-dashboard-v1", "system": {"os": "Linux"},
        "results": [{
            "app": "whisker", "status": "ok", "repetition": 1, "warmup_s": 1,
            "action_delay_s": 0.5, "fps_limit": 144, "vsync": "application-default",
            "csv": "/dashboard.csv", "dependencies": {"whisker": {"rev": "a" * 40}},
            "metrics": {"present_interval_ms": {"median": 20, "p95": 40, "max": 60},
                        "per_frame": [{"elapsed_ns": 1000000, "present_interval_ms": 20},
                                      {"elapsed_ns": 2000000, "present_interval_ms": 60}],
                        "timing_scope": "includes idle time"},
            "actions": [{"action": "click", "started_monotonic_s": 1, "finished_monotonic_s": 2}],
            "timing_validation": {"valid": True},
        }],
    }


class PublishTests(unittest.TestCase):
    def setUp(self):
        self.apps = bench.load_apps()
        self.libraries = [{"id": "whisker", "name": "Whisker"}, {"id": "dioxus-native", "name": "Dioxus Native"}]
        self.source = source_run()

    def convert(self):
        return publish.website_run(self.source, self.apps, self.libraries)

    def test_dashboard_conversion_uses_interval_metrics_without_fps_scores(self):
        run = publish.website_run(dashboard_source(), self.apps, self.libraries)
        result = run["results"][0]
        self.assertEqual(result["scenario"], "dashboard")
        self.assertEqual(result["metrics"], {
            "dashboard_present_median_ms": {"value": 20, "unit": "ms"},
            "dashboard_present_p95_ms": {"value": 40, "unit": "ms"},
            "dashboard_present_max_ms": {"value": 60, "unit": "ms"},
        })
        trace = result["repetitions"][0]["events"][0]["trace"]
        self.assertEqual(trace["timing_scope"], "includes idle time")
        self.assertEqual(trace["actions"][0]["action"], "click")
        self.assertNotIn("renderer_fps", result["metrics"])

    def test_full_window_worst_gap_uses_the_worst_repetition(self):
        source = dashboard_source()
        source["measurement"] = "mangohud-dashboard-v2"
        capture = source["results"][0]
        capture.update(phase_duration_s=2, input_rate_hz=240)
        capture["metrics"] = dashboard.summarize([Frame(0, 1), Frame(1_000_000_000, 1000), Frame(9_000_000_000, 8000)], 0, 8)
        second = copy.deepcopy(capture)
        second["repetition"] = 2
        second["metrics"] = dashboard.summarize([Frame(0, 1), Frame(3_000_000_000, 3000), Frame(9_000_000_000, 6000)], 0, 8)
        source["results"].append(second)
        run = publish.website_run(source, self.apps, self.libraries)
        self.assertEqual(run["results"][0]["metrics"]["dashboard_worst_gap_ms"], {"value": 7000, "unit": "ms"})
        self.assertEqual(run["results"][0]["scenarioVersion"], 2)
        self.assertEqual(run["metadata"]["durationSeconds"], 8)
        self.assertEqual(run["metadata"]["inputRateHz"], 240)
        capture["metrics"] = dashboard.summarize([Frame(0, 1), Frame(9_000_000_000, 9000)], 0, 8)
        run = publish.website_run(source, self.apps, self.libraries)
        self.assertEqual(run["results"][0]["metrics"]["dashboard_worst_gap_ms"]["value"], 8000)

    def test_partial_dashboard_merge_preserves_other_framework_metadata(self):
        previous = self.convert()
        previous["metadata"]["scenarioCollections"] = {"dashboard": {
            "frameworkVersions": {"dioxus-native": "existing version"},
            "frameworkLabels": {"dioxus-native": "Dioxus Native"},
        }}
        history = {"schemaVersion": 1, "runs": [previous]}
        run = publish.website_run(dashboard_source(), self.apps, self.libraries)
        publish.merge_latest(history, run)
        metadata = previous["metadata"]["scenarioCollections"]["dashboard"]
        self.assertEqual(metadata["frameworkVersions"]["dioxus-native"], "existing version")
        self.assertIn("whisker", metadata["frameworkVersions"])

    def test_dashboard_failure_is_not_relabelled_success_and_merge_preserves_scroll(self):
        failed = copy.deepcopy(dashboard_source()["results"][0])
        failed.update(status="failed", error="No capture")
        failed.pop("metrics")
        failed.pop("actions")
        failed.pop("csv")
        failed.pop("dependencies")
        run = publish.website_run({**dashboard_source(), "results": [failed]}, self.apps, self.libraries)
        previous = self.convert()
        history = {"schemaVersion": 1, "runs": [previous]}
        scroll_results = copy.deepcopy(previous["results"])
        publish.merge_latest(history, run)
        self.assertEqual(history["runs"][0]["results"][:1], scroll_results)
        self.assertEqual(history["runs"][0]["results"][-1]["status"], "failed")
        self.assertIn("dashboard", history["runs"][0]["metadata"]["scenarioCollections"])
        self.assertEqual(history["runs"][0]["metadata"]["measurement"], bench.MEASUREMENT)

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
