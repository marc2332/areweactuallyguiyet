import copy
import json
from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from benchmark import dashboard, publish, runner
from benchmark.frames import Frame


def source_run(benchmark="scroll"):
    capture = {
        "app": "whisker", "status": "ok", "repetition": 1, "warmup_s": 1,
        "fps_limit": 144, "vsync": "application-default", "wheel_rate_hz": 144,
        "csv": "/capture.csv", "dependencies": {"whisker": {"rev": "a" * 40}},
        "rows": 1000,
        "metrics": {"measurement_duration_s": 3, "fps": 10, "frames": 30,
                    "max_frame_gap_ms": 100, "frame_interval_ms": {"p95": 100, "p99": 100},
                    "measurement_start_elapsed_ns": 3_000_000_000, "measurement_end_elapsed_ns": 6_000_000_000},
    }
    if benchmark == "dashboard":
        capture.pop("rows")
        capture.update(phase_duration_s=2, input_rate_hz=240, actions=[],
                       memory={"samples_mib": [10, 20, 60]},
                       metrics=dashboard.summarize([Frame(0, 1), Frame(1_000_000_000, 1000),
                                                    Frame(9_000_000_000, 8000)], 0, 8))
    return {
        "id": benchmark, "timestamp": "2026-11-10T20:00:00+00:00",
        "measurement": dashboard.MEASUREMENT if benchmark == "dashboard" else runner.MEASUREMENT,
        "system": {"os": "Linux"}, "results": [capture],
    }


class PublishTests(unittest.TestCase):
    def setUp(self):
        self.apps = runner.load_apps()
        self.libraries = [{"id": "whisker", "name": "Whisker"}, {"id": "dioxus-native", "name": "Dioxus Native"}]

    def convert(self, source):
        return publish.website_run(source, self.apps, self.libraries)

    def test_scroll_aggregates_repetitions_without_mixing_row_counts(self):
        source = source_run()
        original = source["results"][0]
        source["results"] = []
        for rows in (1000, 3000):
            for repetition, fps in ((1, 10), (2, 20)):
                capture = copy.deepcopy(original)
                capture.update(rows=rows, repetition=repetition)
                capture["metrics"].update(fps=fps, frames=fps * 3)
                source["results"].append(capture)
        run = self.convert(source)
        self.assertEqual([result["scenario"] for result in run["results"]], ["scroll-1000", "scroll-3000"])
        for result in run["results"]:
            self.assertEqual(result["metrics"]["renderer_fps"]["value"], 15)
            self.assertEqual(len(result["repetitions"]), 2)
            self.assertEqual(result["scenarioVersion"], 3)

    def test_dashboard_uses_worst_gap_and_sample_weighted_memory(self):
        source = source_run("dashboard")
        second = copy.deepcopy(source["results"][0])
        second.update(repetition=2, memory={"samples_mib": [30]},
                      metrics=dashboard.summarize([Frame(0, 1), Frame(3_000_000_000, 3000),
                                                   Frame(9_000_000_000, 6000)], 0, 8))
        source["results"].append(second)
        result = self.convert(source)["results"][0]
        metrics = result["metrics"]
        self.assertEqual(metrics["dashboard_worst_gap_ms"], {"value": 7000, "unit": "ms"})
        for name, value in (("min", 10), ("mean", 30), ("max", 60)):
            self.assertEqual(metrics[f"dashboard_rss_{name}_mib"], {"value": value, "unit": "MiB"})
        self.assertNotIn("renderer_fps", metrics)
        self.assertEqual(result["repetitions"][0]["events"][0]["trace"]["samples_mib"], [10, 20, 60])

    def test_invalid_captures_are_rejected(self):
        for change in ("identity", "duplicate", "settings", "clock", "rows", "memory"):
            with self.subTest(change=change):
                source = source_run("dashboard" if change == "memory" else "scroll")
                capture = source["results"][0]
                if change == "identity":
                    source["measurement"] = "other-measurement"
                elif change == "clock":
                    capture["metrics"]["fps"] = 100
                elif change == "rows":
                    capture["rows"] = 0
                elif change == "memory":
                    capture["memory"]["samples_mib"] = [float("nan")]
                else:
                    second = copy.deepcopy(capture)
                    if change == "settings":
                        second.update(repetition=2, fps_limit=60)
                    source["results"].append(second)
                with self.assertRaises(ValueError):
                    self.convert(source)

    def test_failed_variants_remain_unavailable_with_their_labels(self):
        source = source_run()
        source["results"].append({"app": "dioxus-native-vello-hybrid", "status": "failed",
                                  "repetition": 1, "error": "No capture"})
        run = self.convert(source)
        self.assertEqual(run["metadata"]["frameworkBaseIds"]["dioxus-native-vello-hybrid"], "dioxus-native")
        self.assertEqual(run["metadata"]["frameworkLabels"]["dioxus-native-vello-hybrid"], "Dioxus Native Vello Hybrid")
        self.assertEqual(run["results"][1]["status"], "failed")
        self.assertEqual(run["results"][1]["metrics"], {})

    def test_latest_refresh_preserves_other_workloads_and_older_collections(self):
        previous = self.convert(source_run())
        older = copy.deepcopy(previous)
        older.update(id="older", collectedAt="2026-11-09T20:00:00+00:00")
        history = {"schemaVersion": 1, "runs": [older, previous]}
        dashboard_run = self.convert(source_run("dashboard"))
        publish.merge_latest(history, dashboard_run)
        self.assertEqual(previous["results"][0]["scenario"], "scroll-1000")
        dashboard_results = copy.deepcopy([result for result in previous["results"] if result["scenario"] == "dashboard"])
        dashboard_metadata = copy.deepcopy(previous["metadata"]["scenarioCollections"]["dashboard"])
        source = source_run()
        source["results"][0]["rows"] = 3000
        replacement = self.convert(source)
        for _ in range(2):
            publish.merge_latest(history, replacement)
        self.assertEqual(history["runs"][0], older)
        self.assertEqual([result["scenario"] for result in previous["results"]], ["dashboard", "scroll-3000"])
        self.assertEqual([result for result in previous["results"] if result["scenario"] == "dashboard"], dashboard_results)
        self.assertEqual(previous["metadata"]["scenarioCollections"]["dashboard"], dashboard_metadata)

    def test_save_preserves_prior_collections_and_rejects_duplicates(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "history.json"
            previous = self.convert(source_run())
            previous["id"] = "previous"
            path.write_text(json.dumps({"schemaVersion": 1, "runs": [previous]}))
            run = self.convert(source_run())
            publish.save(path, run)
            history = json.loads(path.read_text())
            self.assertEqual(history["runs"], [previous, run])
            with self.assertRaisesRegex(ValueError, "already been published"):
                publish.save(path, run)
            self.assertEqual(json.loads(path.read_text()), history)
