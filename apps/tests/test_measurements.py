from pathlib import Path
import sys
import tempfile
import unittest
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from benchmark import dashboard
from benchmark.frames import CaptureWindow, Frame, FrameLog, parse_csv, summarize


class MeasurementTests(unittest.TestCase):
    def test_capture_waits_for_a_fresh_frame_and_a_complete_window(self):
        phase = CaptureWindow(0.02, 2, 5, 0)
        frames = [Frame(0, 10)]
        self.assertTrue(phase.update(frames, 2, 120))
        self.assertIsNone(phase.start_ns)
        frames.append(Frame(10_000_000, 10))
        self.assertTrue(phase.update(frames, 3, 180))
        frames.append(Frame(20_000_000, 10))
        self.assertTrue(phase.update(frames, 3.01, 181))
        frames.append(Frame(30_000_000, 10))
        self.assertFalse(phase.update(frames, 3.02, 182))
        self.assertEqual(phase.start_ns, 10_000_000)
        self.assertEqual(phase.end_events - phase.start_events, 2)
        self.assertEqual(summarize(frames, phase.start_ns, 0.02)["fps"], 100)

    def test_capture_times_out_instead_of_ranking_incomplete_data(self):
        with self.assertRaisesRegex(RuntimeError, "No fresh validated frame"):
            CaptureWindow(1, 2, 5, 0).update([], 7.1, 420)
        phase = CaptureWindow(2, 1, 5, 0)
        frames = [Frame(0, 10), Frame(10_000_000, 10)]
        phase.update(frames, 1, 60)
        with self.assertRaisesRegex(RuntimeError, "measurement endpoint"):
            phase.update(frames, 8.1, 486)
        with self.assertRaisesRegex(ValueError, "does not bracket"):
            summarize(frames, 0, 2)

    def test_scroll_counts_boundaries_once_and_keeps_stalls(self):
        for timestamps, start, duration, fps, gap in (
            ([0, 100, 200, 300], 100, 0.1, 10, 100),
            ([0, 1500, 2500], 0, 2, 0.5, 1500),
            ([0, 3000], 0, 2, 0, 2000),
        ):
            with self.subTest(timestamps=timestamps):
                frames = [Frame(value * 1_000_000, 10) for value in timestamps]
                metrics = summarize(frames, start * 1_000_000, duration)
                self.assertEqual(metrics["fps"], fps)
                self.assertEqual(metrics["max_frame_gap_ms"], gap)
                if fps == 0:
                    self.assertIsNone(metrics["frame_interval_ms"])

    def test_live_csv_waits_for_complete_rows_and_import_anchors_to_a_frame(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.csv"
            path.write_text("os,cpu\nLinux,CPU\nfps,frametime,elapsed\n10,100,0\n10,100,")
            with FrameLog(path) as trace:
                trace.poll()
                self.assertEqual(len(trace.frames), 1)
                with path.open("a") as file:
                    file.write("100000000\n10,100,200000000\n")
                trace.poll()
                trace.poll()
                self.assertEqual(len(trace.frames), 3)
                trace.finish()
            metrics = parse_csv(path, 0.1, start=0.05)
            self.assertEqual(metrics["measurement_start_elapsed_ns"], 100_000_000)
            self.assertEqual(metrics["frames"], 1)

    def test_csv_rejects_malformed_rows_and_corrupt_clocks(self):
        for rows in (
            "", "100,nan,1000\n", "100,0,1000\n", "inf,1,1000\n",
            "100,1,-1\n", "100,1,1000\n100,1,1000\n",
            "100,1,2000\n100,1,1000\n", "100,1\n", "100,1,1000,extra\n",
            "100,1,1000000\n100,1,", "100,10,10000000\n0.00004,26497000,1020000000\n",
        ):
            with self.subTest(rows=rows), tempfile.TemporaryDirectory() as directory:
                path = Path(directory) / "capture.csv"
                path.write_text("fps,frametime,elapsed\n" + rows)
                with FrameLog(path) as trace, self.assertRaises(ValueError):
                    trace.finish()

    def test_elapsed_only_mode_still_validates_timestamps(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "capture.csv"
            path.write_text("fps,frametime,elapsed\n1,40000000,0\n1,40000000,1000000000\n1,40000000,4000000000\n")
            with FrameLog(path) as trace, self.assertRaisesRegex(ValueError, "disagrees"):
                trace.finish()
            with FrameLog(path, validate_reported_frametime=False) as trace:
                trace.finish()
                self.assertEqual(summarize(trace.frames, 0, 3)["fps"], 1 / 3)
                self.assertEqual(trace.reported_timing_mismatches, 2)
            path.write_text("fps,frametime,elapsed\n1,40000000,100\n1,40000000,100\n")
            with FrameLog(path, validate_reported_frametime=False) as trace:
                with self.assertRaisesRegex(ValueError, "strictly increasing"):
                    trace.finish()

    def test_dashboard_keeps_trailing_silence_and_phase_transition_gaps(self):
        frames = [Frame(0, 1), Frame(1_000_000_000, 1000), Frame(1_010_000_000, 10),
                  Frame(3_010_000_000, 2000), Frame(9_100_000_000, 6090)]
        metrics = dashboard.summarize(frames, 1_000_000_000, 8)
        self.assertEqual(metrics["present_interval_ms"]["max"], 2000)
        self.assertEqual(metrics["worst_presentation_gap_ms"], 5990)
        self.assertEqual(metrics["trailing_gap_ms"], 5990)
        with self.assertRaisesRegex(ValueError, "bracket"):
            dashboard.summarize(frames[:-1], 1_000_000_000, 8)
        idle = dashboard.summarize([Frame(0, 1), Frame(9_000_000_000, 9000)], 0, 8)
        self.assertEqual(idle["worst_presentation_gap_ms"], 8000)
        self.assertIsNone(idle["present_interval_ms"])
        frames = [Frame(index * 100_000_000, 100) for index in range(82) if not 20 <= index < 30]
        self.assertEqual(dashboard.summarize(frames, 0, 8)["worst_presentation_gap_ms"], 1100)

    def test_dashboard_phases_keep_their_schedule_with_slow_input(self):
        for action_duration in (0, 0.03):
            with self.subTest(action_duration=action_duration):
                clock = [0.0]
                input = mock.Mock()

                def advance(duration):
                    clock[0] += duration

                input.perform.side_effect = lambda _: advance(action_duration)
                with mock.patch.object(dashboard.time, "monotonic", side_effect=lambda: clock[0]), \
                     mock.patch.object(dashboard.time, "sleep", side_effect=advance):
                    workload = dashboard.exercise(input, mock.Mock(), 240)
                self.assertLess(workload["actual_duration_s"], 8.04)
                self.assertEqual(workload["duration_s"], 8)
                self.assertEqual([phase["phase"] for phase in workload["phases"]],
                                 ["typing", "refresh", "toggle", "scroll"])
                self.assertEqual([phase["start_offset_s"] for phase in workload["phases"]], [0, 2, 4, 6])
                for index, expected in enumerate(({"type-a", "backspace"}, {"refresh"}, {"toggle"}, {"scroll-down", "scroll-up"})):
                    actions = [action["action"] for action in workload["actions"]
                               if index * 2 <= action["started_monotonic_s"] < (index + 1) * 2]
                    self.assertEqual(set(actions), expected)
