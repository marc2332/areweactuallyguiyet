import contextlib
import io
import json
from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bench
import dashboard
from frames import Frame


class DashboardTests(unittest.TestCase):
    def test_worst_gap_includes_trailing_silence_and_excludes_warmup(self):
        frames = [Frame(0, 1), Frame(1_000_000_000, 1000), Frame(1_010_000_000, 10),
                  Frame(3_010_000_000, 2000), Frame(9_100_000_000, 6090)]
        metrics = dashboard.summarize(frames, 1_000_000_000, 8)
        self.assertEqual(metrics["present_interval_ms"]["max"], 2000)
        self.assertEqual(metrics["worst_presentation_gap_ms"], 5990)
        self.assertEqual(metrics["trailing_gap_ms"], 5990)
        self.assertEqual(metrics["measurement_end_elapsed_ns"], 9_000_000_000)
        self.assertEqual(len(metrics["per_frame"]), 2)
        with self.assertRaisesRegex(ValueError, "bracket"):
            dashboard.summarize(frames[:-1], 1_000_000_000, 8)

    def test_no_presentations_in_the_window_means_an_eight_second_gap(self):
        metrics = dashboard.summarize([Frame(0, 1), Frame(9_000_000_000, 9000)], 0, 8)
        self.assertEqual(metrics["worst_presentation_gap_ms"], 8000)
        self.assertEqual(metrics["per_frame"], [])
        self.assertIsNone(metrics["present_interval_ms"])

    def test_phase_transition_gaps_are_not_removed(self):
        frames = [Frame(index * 100_000_000, 100) for index in range(82)
                  if not 20 <= index < 30]
        metrics = dashboard.summarize(frames, 0, 8)
        self.assertEqual(metrics["worst_presentation_gap_ms"], 1100)

    def test_four_consecutive_two_second_phases(self):
        clock = [0.0]
        input = mock.Mock()

        def sleep(duration):
            clock[0] += duration

        with mock.patch("dashboard.time.monotonic", side_effect=lambda: clock[0]), \
             mock.patch("dashboard.time.sleep", side_effect=sleep):
            workload = dashboard.exercise(input, mock.Mock(), 20)
        self.assertAlmostEqual(clock[0], 8)
        self.assertEqual(workload["duration_s"], 8)
        self.assertEqual([phase["phase"] for phase in workload["phases"]], ["typing", "refresh", "toggle", "scroll"])
        self.assertEqual([phase["start_offset_s"] for phase in workload["phases"]], [0, 2, 4, 6])
        self.assertTrue(all(phase["duration_s"] == 2 for phase in workload["phases"]))
        for phase_index, expected in enumerate(({"type-a", "backspace"}, {"refresh"}, {"toggle"}, {"scroll-down", "scroll-up"})):
            actions = [action for action in workload["actions"]
                       if phase_index * 2 <= action["started_monotonic_s"] < (phase_index + 1) * 2]
            self.assertEqual({action["action"] for action in actions}, expected)
            self.assertLess(actions[0]["started_monotonic_s"] - phase_index * 2, 0.001)

    def test_slow_input_does_not_extend_every_phase(self):
        clock = [0.0]
        input = mock.Mock()

        def perform(_):
            clock[0] += 0.03

        input.perform.side_effect = perform
        with mock.patch("dashboard.time.monotonic", side_effect=lambda: clock[0]):
            workload = dashboard.exercise(input, mock.Mock(), 240)
        self.assertLess(workload["actual_duration_s"], 8.04)
        self.assertEqual(workload["duration_s"], 8)

    def test_pointer_coordinates_follow_desktop_scaling(self):
        input = dashboard.DashboardInput.__new__(dashboard.DashboardInput)
        input.size = (1083, 758)
        input.library, input.xlib = mock.Mock(), mock.Mock()
        input.library.xdo_move_mouse_relative_to_window.return_value = 0
        input.connection, input.display, input.window = 1, 2, 3
        input.move(496, 90)
        input.library.xdo_move_mouse_relative_to_window.assert_called_once_with(1, 3, 537, 97)

    def test_parse_dashboard_has_no_duration_and_preserves_intervals(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.csv"
            path.write_text("fps,frametime,elapsed\n100,10,10000000\n5,200,210000000\n")
            with mock.patch.object(sys, "argv", ["bench.py", "parse", str(path), "--app", "egui",
                                                "--benchmark", "dashboard"]), \
                 contextlib.redirect_stdout(io.StringIO()) as output:
                self.assertEqual(bench.main(), 0)
            run = json.loads(output.getvalue())
            self.assertEqual(run["measurement"], dashboard.MEASUREMENT)
            self.assertEqual(len(run["results"][0]["metrics"]["per_frame"]), 1)

    def test_dashboard_selects_feature_and_separate_history(self):
        with mock.patch.object(sys, "argv", ["bench.py", "run", "egui", "--benchmark", "dashboard"]), \
             mock.patch.object(bench, "run_benchmarks", return_value=0) as run:
            self.assertEqual(bench.main(), 0)
            arguments = run.call_args.args[0]
            self.assertEqual(arguments.benchmark, "dashboard")
            self.assertEqual(arguments.history.name, "dashboard-history.json")
            self.assertEqual(arguments.rate, 240)
            self.assertEqual(arguments.fps_limit, 0)
            self.assertTrue(arguments.no_vsync)

    def test_build_keeps_features_and_profiles_separate(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            manifest = root / "Cargo.toml"
            manifest.write_text('[features]\ndefault = ["scroll"]\nscroll = []\ndashboard = []\n')
            binary = root / "release" / "example"
            binary.parent.mkdir()
            binary.write_bytes(b"scroll executable")
            app = {"id": "example", "manifest": manifest}
            metadata = {"target_directory": str(root), "packages": [{"manifest_path": str(manifest),
                        "targets": [{"name": "example", "kind": ["bin"]}]}]}
            with mock.patch.object(bench.subprocess, "run") as build, \
                 mock.patch.object(bench.subprocess, "check_output", return_value=json.dumps(metadata)):
                selected = bench.build(app, root, False, "scroll")
                command = build.call_args.args[0]
                self.assertEqual(command[-3:], ["--no-default-features", "--features", "scroll"])
                self.assertEqual(selected.read_bytes(), b"scroll executable")
                with self.assertRaisesRegex(ValueError, "Missing scroll production binary"):
                    bench.build(app, root, True, profile="production")
                production_binary = root / "production" / "example"
                production_binary.parent.mkdir()
                production_binary.write_bytes(b"production executable")
                production = bench.build(app, root, False, profile="production")
                self.assertEqual(build.call_args.args[0][2:4], ["--profile", "production"])
                self.assertNotEqual(production, selected)
                self.assertEqual(production.read_bytes(), b"production executable")
                self.assertEqual(bench.build(app, root, True, profile="production"), production)
                binary.write_bytes(b"dashboard executable")
                self.assertEqual(bench.build(app, root, True, "scroll").read_bytes(), b"scroll executable")
                with self.assertRaisesRegex(ValueError, "Missing dashboard release binary"):
                    bench.build(app, root, True, "dashboard")
                manifest.write_text('[features]\ndefault = ["scroll", "skia"]\n')
                bench.build(app, root, False, "dashboard")
                self.assertEqual(build.call_args.args[0][-1], "dashboard,skia")


if __name__ == "__main__":
    unittest.main()
