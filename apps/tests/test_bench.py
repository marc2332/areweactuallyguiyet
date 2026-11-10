import contextlib
import ctypes
import io
import json
from pathlib import Path
import subprocess
import sys
import tempfile
from types import SimpleNamespace
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import bench


class BenchmarkTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.csv = self.root / "capture.csv"
        self.history = self.root / "history.json"

    def test_discovery_includes_hybrid_proxy_and_whisker(self):
        apps = bench.load_apps()
        for name in ("dioxus-native-vello-hybrid", "whisker"):
            self.assertEqual(apps[name]["renderer"], "vulkan")
            self.assertTrue(apps[name]["manifest"].is_file())
        proxy = bench.tomllib.loads(apps["dioxus-native-vello-hybrid"]["manifest"].read_text())
        dependency = proxy["dependencies"]["dioxus-native-scroll"]
        self.assertEqual(dependency["path"], "../dioxus-native")
        self.assertFalse(dependency["default-features"])
        self.assertEqual(dependency["features"], ["vello-hybrid"])
        original = bench.tomllib.loads(apps["dioxus-native"]["manifest"].read_text())
        self.assertEqual(original["features"]["default"], ["skia"])

    def write_csv(self, rows, prefix="os,cpu\nLinux,Example CPU\n"):
        self.csv.write_text(prefix + "fps,frametime,elapsed\n" + rows)

    def cli(self, *arguments):
        return subprocess.run(
            [sys.executable, str(bench.DIRECTORY / "bench.py"), *arguments],
            text=True, capture_output=True, check=False,
        )

    def test_statistics_use_a_bracketed_timestamp_window(self):
        self.write_csv("1000,1,1000000\n250,4,5000000\n500,2,7000000\n200,5,12000000\n")
        result = bench.parse_csv(self.csv, 0.01)
        self.assertEqual(result["frames"], 2)
        self.assertEqual(result["fps"], 200)
        self.assertEqual(result["measurement_start_elapsed_ns"], 1000000)
        self.assertEqual(result["measurement_end_elapsed_ns"], 11000000)
        self.assertEqual(result["last_sample_elapsed_s"], 0.012)
        self.assertEqual(result["trailing_gap_ms"], 4)
        self.assertEqual(result["max_frame_gap_ms"], 4)
        self.assertEqual(result["frame_interval_ms"], {"median": 3, "p95": 4, "p99": 4, "max": 4})

    def test_versioned_header_is_found(self):
        self.write_csv("100,10,10000000\n1,1000,1010000000\n", prefix="v1\n0.6.9\nSYSTEM INFO\nos,cpu\nLinux,CPU\nFRAME METRICS\n")
        self.assertEqual(bench.parse_csv(self.csv, 1)["frames"], 1)

    def test_bad_frames_are_rejected_not_dropped(self):
        for rows in (
            "100,nan,1000\n", "100,0,1000\n", "inf,1,1000\n", "100,1,-1\n",
            "100,1,1000\n100,1,1000\n", "100,1,2000\n100,1,1000\n",
            "100,1\n", "100,1,1000,extra\n", "100,nope,1000\n", "",
        ):
            with self.subTest(rows=rows):
                self.write_csv(rows)
                with self.assertRaises(ValueError):
                    bench.parse_csv(self.csv, 1)

    def test_summary_csv_is_not_a_frame_capture(self):
        self.csv.write_text("Average FPS,GPU Load\n144,20\n")
        with self.assertRaisesRegex(ValueError, "No MangoHud frame table"):
            bench.parse_csv(self.csv, 15)

    def test_default_input_rate_follows_the_fps_cap(self):
        for extra, expected in (([], 120), (["--rate", "60"], 60)):
            with self.subTest(extra=extra), \
                 mock.patch.object(sys, "argv", ["bench.py", "run", "freya", "--fps-limit", "120", *extra]), \
                 mock.patch.object(bench, "load_apps", return_value={"freya": {}}), \
                 mock.patch.object(bench, "run_benchmarks", return_value=0) as run:
                self.assertEqual(bench.main(), 0)
                self.assertEqual(run.call_args.args[0].rate, expected)

    def test_uncapped_run_keeps_a_nonzero_default_input_rate(self):
        with mock.patch.object(sys, "argv", ["bench.py", "run", "freya", "--fps-limit", "0", "--no-vsync"]), \
             mock.patch.object(bench, "load_apps", return_value={"freya": {}}), \
             mock.patch.object(bench, "run_benchmarks", return_value=0) as run:
            self.assertEqual(bench.main(), 0)
            arguments = run.call_args.args[0]
            self.assertEqual(arguments.fps_limit, 0)
            self.assertEqual(arguments.rate, 144)
            self.assertTrue(arguments.no_vsync)

    def test_invalid_duration_is_rejected(self):
        self.write_csv("100,10,10000000\n")
        for duration in (0, -1, float("nan"), float("inf")):
            with self.subTest(duration=duration), self.assertRaises(ValueError):
                bench.parse_csv(self.csv, duration)

    def test_cli_parse_does_not_create_history_by_default(self):
        self.write_csv("100,10,10000000\n1,1000,1010000000\n")
        destination = self.root / "absent" / "history.json"
        result = self.cli("parse", str(self.csv), "--app", "freya", "--duration", "1", "--history", str(destination))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout)["results"][0]["metrics"]["frames"], 1)
        self.assertFalse(destination.parent.exists())

    def test_without_save_existing_history_is_untouched(self):
        self.write_csv("100,10,10000000\n1,1000,1010000000\n")
        self.history.write_text("existing data, not even JSON")
        result = self.cli("parse", str(self.csv), "--app", "freya", "--duration", "1", "--history", str(self.history))
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.history.read_text(), "existing data, not even JSON")
        self.assertFalse(self.history.with_suffix(".json.lock").exists())

    def test_cli_save_appends_only_when_requested(self):
        self.write_csv("100,10,10000000\n1,1000,1010000000\n")
        for _ in range(2):
            result = self.cli("parse", str(self.csv), "--app", "freya", "--duration", "1", "--save", "--history", str(self.history))
            self.assertEqual(result.returncode, 0, result.stderr)
        history = json.loads(self.history.read_text())
        self.assertEqual(history["measurement"], bench.MEASUREMENT)
        self.assertEqual(len(history["runs"]), 2)

    def test_old_history_is_not_overwritten_or_mixed(self):
        original = '{"schemaVersion":1,"runs":[]}'
        self.history.write_text(original)
        with self.assertRaisesRegex(ValueError, "Not an apps"):
            bench.save_history(self.history, bench.new_run(["parse"]))
        self.assertEqual(self.history.read_text(), original)

    def test_v1_history_is_not_mixed_with_corrected_windows(self):
        original = json.dumps({"schemaVersion": 1, "measurement": "mangohud-scroll-v1", "runs": []})
        self.history.write_text(original)
        with self.assertRaisesRegex(ValueError, "Not an apps"):
            bench.save_history(self.history, bench.new_run(["run"]))
        self.assertEqual(self.history.read_text(), original)

    def test_rejects_legacy_history_and_unaccepted_identity(self):
        original = '{"schemaVersion":1,"runs":[]}'
        self.history.write_text(original)
        with self.assertRaisesRegex(ValueError, "Not an apps"):
            bench.save_history(self.history, bench.new_run(["run"]))
        self.assertEqual(self.history.read_text(), original)
        destination = self.root / "absent" / "history.json"
        with self.assertRaisesRegex(ValueError, "measurement identity"):
            bench.save_history(destination, {**bench.new_run(["test"]), "measurement": "unknown"})
        self.assertFalse(destination.parent.exists())

    def test_non_object_history_is_rejected_without_replacement(self):
        self.history.write_text("[]")
        with self.assertRaisesRegex(ValueError, "Not an apps"):
            bench.save_history(self.history, bench.new_run(["run"]))
        self.assertEqual(self.history.read_text(), "[]")

    def test_mixed_run_identities_inside_history_are_rejected(self):
        original = json.dumps({"schemaVersion": 1, "measurement": bench.MEASUREMENT,
                               "runs": [{**bench.new_run(["run"]), "measurement": "other-v1"}]})
        self.history.write_text(original)
        with self.assertRaisesRegex(ValueError, "Not an apps"):
            bench.save_history(self.history, bench.new_run(["run"]))
        self.assertEqual(self.history.read_text(), original)

    def test_bad_csv_is_not_saved(self):
        self.write_csv("100,nan,1000000\n")
        result = self.cli("parse", str(self.csv), "--app", "freya", "--duration", "1", "--save", "--history", str(self.history))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.history.exists())

    def test_unknown_app_fails_without_saving(self):
        self.write_csv("100,10,10000000\n1,1000,1010000000\n")
        result = self.cli("parse", str(self.csv), "--app", "unknown", "--duration", "1", "--save", "--history", str(self.history))
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse(self.history.exists())

    def test_finish_without_save_does_not_call_history_writer(self):
        arguments = mock.Mock(save=False)
        with mock.patch.object(bench, "save_history") as save, contextlib.redirect_stdout(io.StringIO()):
            bench.finish(bench.new_run(["run", "freya"]), arguments)
        save.assert_not_called()

    def test_save_does_not_replace_malformed_history(self):
        self.history.write_text("not JSON")
        with self.assertRaises(ValueError):
            bench.save_history(self.history, bench.new_run(["run"]))
        self.assertEqual(self.history.read_text(), "not JSON")

    def test_interruption_survives_failed_stop_and_never_saves(self):
        arguments = SimpleNamespace(
            apps=["freya"], output=self.root / "results", no_build=True,
            repetitions=1, fps_limit=144, no_vsync=False, startup_timeout=1,
            warmup=2, duration=15, rate=60, save=True, history=self.history,
        )
        app = {"id": "freya", "title": "Freya Scroll 2000", "renderer": "opengl", "env": {}}
        process = mock.Mock()
        process.poll.return_value = None
        wheel = mock.MagicMock()
        wheel.scroll.side_effect = KeyboardInterrupt()
        control = mock.MagicMock()
        control.sendall.side_effect = [None, BrokenPipeError("disconnected")]
        with mock.patch.object(bench.shutil, "which", return_value="/usr/bin/tool"), \
             mock.patch.dict(bench.os.environ, {"DISPLAY": ":0"}), \
             mock.patch.object(bench, "build", return_value=self.root / "binary"), \
             mock.patch.object(bench.subprocess, "Popen", return_value=process), \
             mock.patch.object(bench, "wait_for_window", return_value=1), \
             mock.patch.object(bench, "WheelInput") as input_factory, \
             mock.patch.object(bench, "connect_control") as control_factory, \
             mock.patch.object(bench, "finish") as finish, \
             mock.patch.object(bench.time, "sleep"):
            input_factory.return_value.__enter__.return_value = wheel
            control_factory.return_value.__enter__.return_value = control
            with self.assertRaises(KeyboardInterrupt):
                bench.run_benchmarks(arguments, {"freya": app})
        finish.assert_not_called()
        process.terminate.assert_called_once()
        self.assertFalse(self.history.exists())

    def test_low_wheel_rate_does_not_overrun_phase(self):
        wheel = bench.WheelInput.__new__(bench.WheelInput)
        wheel.library = mock.Mock()
        wheel.library.xdo_click_window.return_value = 0
        wheel.xlib = mock.Mock()
        wheel.connection, wheel.display = 1, 1
        wheel.events_sent = 0
        clock = [0.0]

        def sleep(duration):
            clock[0] += duration

        with mock.patch("scroll.time.monotonic", side_effect=lambda: clock[0]), \
             mock.patch("scroll.time.sleep", side_effect=sleep):
            count = wheel.scroll(2, 0.1)
        self.assertEqual(count, 1)
        self.assertEqual(clock[0], 2)

    def test_scroll_stops_when_the_frame_window_is_complete(self):
        wheel = bench.WheelInput.__new__(bench.WheelInput)
        wheel.library, wheel.xlib = mock.Mock(), mock.Mock()
        wheel.connection, wheel.display, wheel.events_sent = 1, 1, 0
        count = wheel.scroll(10, 144, tick=lambda: False)
        self.assertEqual(count, 0)
        wheel.library.xdo_click_window.assert_not_called()

    def test_window_is_not_used_before_it_is_mapped(self):
        process = mock.Mock(pid=123)
        process.poll.return_value = None
        answers = [
            '0x10 "Example Scroll 2000"', '_NET_WM_PID = 123', 'Map State: IsUnMapped',
            '0x10 "Example Scroll 2000"', '_NET_WM_PID = 123', 'Map State: IsViewable',
        ]
        with mock.patch.object(bench.subprocess, "check_output", side_effect=answers) as query, \
             mock.patch.object(bench.time, "sleep"):
            window = bench.wait_for_window(process, "Example Scroll 2000", 5)
        self.assertEqual(window, 0x10)
        self.assertEqual(query.call_count, 6)

    def test_xres_identifies_windows_without_pid_property(self):
        process = mock.Mock(pid=123)
        process.poll.return_value = None
        answers = ['0x10 "Example Scroll 2000"', '_NET_WM_PID: not found.', 'Map State: IsViewable']
        with mock.patch.object(bench.subprocess, "check_output", side_effect=answers), \
             mock.patch.object(bench, "window_client_pid", return_value=123) as identify:
            window = bench.wait_for_window(process, "Example Scroll 2000", 5)
        self.assertEqual(window, 0x10)
        identify.assert_called_once_with(0x10)

    def test_same_title_from_another_pid_is_not_used(self):
        process = mock.Mock(pid=123)
        process.poll.return_value = None
        with mock.patch.object(bench.subprocess, "check_output", side_effect=[
            '0x10 "Example Scroll 2000"', '_NET_WM_PID = 999',
        ]), mock.patch.object(bench, "window_client_pid") as identify, \
             mock.patch.object(bench.time, "monotonic", side_effect=[0, 0, 10]), \
             mock.patch.object(bench.time, "sleep"):
            with self.assertRaisesRegex(RuntimeError, "owned by PID 123"):
                bench.wait_for_window(process, "Example Scroll 2000", 5)
        identify.assert_not_called()

    def test_input_waits_for_activation_and_moves_pointer(self):
        xlib, library = mock.Mock(), mock.Mock()
        xlib.XOpenDisplay.return_value = 1
        library.xdo_new_with_opened_display.return_value = 2
        library.xdo_activate_window.return_value = 0
        library.xdo_wait_for_window_active.side_effect = [1, 0]
        library.xdo_move_mouse_relative_to_window.return_value = 0

        def size(_connection, _window, width, height):
            ctypes.cast(width, ctypes.POINTER(ctypes.c_uint)).contents.value = 1000
            ctypes.cast(height, ctypes.POINTER(ctypes.c_uint)).contents.value = 700
            return 0

        library.xdo_get_window_size.side_effect = size
        with mock.patch("scroll.ctypes.CDLL", side_effect=[xlib, library]), \
             mock.patch("scroll.time.monotonic", return_value=0), \
             mock.patch("scroll.time.sleep"):
            with bench.WheelInput(10):
                pass
        self.assertEqual(library.xdo_activate_window.call_count, 2)
        self.assertEqual(library.xdo_move_mouse_relative_to_window.call_args_list, [
            mock.call(2, 10, 499, 350), mock.call(2, 10, 500, 350),
        ])
        library.xdo_free.assert_called_once_with(2)

    def test_no_vsync_and_no_limiter_reach_mangohud(self):
        process = mock.Mock()
        process.poll.return_value = None
        arguments = SimpleNamespace(fps_limit=0, no_vsync=True, startup_timeout=1)
        app = {"id": "example", "title": "Example Scroll 2000", "renderer": "opengl", "env": {}}
        with mock.patch.object(bench.subprocess, "Popen", return_value=process) as launch, \
             mock.patch.object(bench, "wait_for_window", side_effect=RuntimeError("startup failed")):
            with self.assertRaisesRegex(RuntimeError, "startup failed"):
                bench.capture(app, self.root / "binary", self.root / "capture", arguments)
        config = set(launch.call_args.kwargs["env"]["MANGOHUD_CONFIG"].split(","))
        self.assertTrue({"fps_limit=0", "gl_vsync=0", "vsync=1"}.issubset(config))

    def test_capture_startup_failure_terminates_owned_process(self):
        process = mock.Mock()
        process.poll.return_value = None
        arguments = mock.Mock(fps_limit=144, no_vsync=False, startup_timeout=1)
        app = {"id": "example", "title": "Example Scroll 2000", "renderer": "opengl", "env": {}}
        with mock.patch.object(bench.subprocess, "Popen", return_value=process), \
             mock.patch.object(bench, "wait_for_window", side_effect=RuntimeError("startup failed")):
            with self.assertRaisesRegex(RuntimeError, "startup failed"):
                bench.capture(app, self.root / "binary", self.root / "capture", arguments)
        process.terminate.assert_called_once()
        process.wait.assert_called_once_with(timeout=5)


if __name__ == "__main__":
    unittest.main()
