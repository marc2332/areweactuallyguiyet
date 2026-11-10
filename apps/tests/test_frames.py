from pathlib import Path
import sys
import tempfile
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from frames import CaptureWindow, Frame, FrameLog, parse_csv, summarize


class FrameWindowTests(unittest.TestCase):
    def test_warmup_requires_a_fresh_frame_not_a_stale_sample(self):
        phase = CaptureWindow(1, 2, 5, 0)
        frames = [Frame(10000000, 10), Frame(20000000, 10)]
        self.assertTrue(phase.update(frames, 1, 60))
        self.assertTrue(phase.update(frames, 2, 120))
        self.assertIsNone(phase.start_ns)
        frames.append(Frame(30000000, 10))
        self.assertTrue(phase.update(frames, 2.1, 126))
        self.assertEqual(phase.start_ns, 30000000)
        self.assertEqual(phase.start_events, 126)

    def test_delayed_logging_does_not_shorten_measurement(self):
        phase = CaptureWindow(0.02, 2, 5, 0)
        self.assertTrue(phase.update([], 2, 120))
        frames = [Frame(3000000000, 10)]
        self.assertTrue(phase.update(frames, 3, 180))
        self.assertIsNone(phase.start_ns)
        frames.append(Frame(3010000000, 10))
        self.assertTrue(phase.update(frames, 3.01, 181))
        frames.append(Frame(3020000000, 10))
        self.assertTrue(phase.update(frames, 3.02, 182))
        frames.append(Frame(3030000000, 10))
        self.assertFalse(phase.update(frames, 3.03, 183))
        metrics = summarize(frames, phase.start_ns, 0.02)
        self.assertEqual(metrics["frames"], 2)
        self.assertEqual(metrics["fps"], 100)
        self.assertEqual(metrics["measurement_duration_s"], 0.02)
        self.assertEqual(phase.end_events, 183)

    def test_no_fresh_frames_fails_readiness_instead_of_ranking_idle(self):
        phase = CaptureWindow(1, 2, 5, 0)
        with self.assertRaisesRegex(RuntimeError, "No fresh validated frame"):
            phase.update([], 7.1, 420)

    def test_stalled_endpoint_does_not_move_the_measurement_window(self):
        phase = CaptureWindow(2, 1, 5, 0)
        frames = [Frame(0, 10), Frame(10000000, 10)]
        phase.update(frames, 1, 60)
        with self.assertRaisesRegex(RuntimeError, "measurement endpoint"):
            phase.update(frames, 8.1, 486)
        self.assertEqual(phase.start_ns, 10000000)

    def test_real_long_stall_is_included(self):
        frames = [Frame(0, 10), Frame(1500000000, 1500), Frame(2500000000, 1000)]
        metrics = summarize(frames, 0, 2)
        self.assertEqual(metrics["fps"], 0.5)
        self.assertEqual(metrics["frame_interval_ms"]["max"], 1500)
        self.assertEqual(metrics["max_frame_gap_ms"], 1500)
        self.assertEqual(metrics["trailing_gap_ms"], 500)

    def test_zero_frames_in_bracketed_window_is_zero_not_missing(self):
        frames = [Frame(0, 10), Frame(3000000000, 3000)]
        metrics = summarize(frames, 0, 2)
        self.assertEqual(metrics["fps"], 0)
        self.assertIsNone(metrics["frame_interval_ms"])
        self.assertEqual(metrics["max_frame_gap_ms"], 2000)

    def test_missing_tail_coverage_is_rejected(self):
        with self.assertRaisesRegex(ValueError, "does not bracket"):
            summarize([Frame(0, 10), Frame(1000000000, 1000)], 0, 2)

    def test_boundary_frame_is_counted_once_and_preroll_is_excluded(self):
        frames = [Frame(0, 100), Frame(100000000, 100), Frame(200000000, 100), Frame(300000000, 100)]
        metrics = summarize(frames, 100000000, 0.1)
        self.assertEqual(metrics["frames"], 1)
        self.assertEqual(metrics["fps"], 10)
        self.assertEqual(metrics["trailing_gap_ms"], 0)


class FrameLogTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.path = Path(self.directory.name) / "capture.csv"

    def test_elapsed_only_counts_frames_without_using_broken_reported_intervals(self):
        self.path.write_text("fps,frametime,elapsed\n0.000025,40000000,0\n0.000025,40000000,1000000000\n0.000025,40000000,2000000000\n0.000025,40000000,4000000000\n")
        with FrameLog(self.path) as trace:
            with self.assertRaisesRegex(ValueError, "disagrees"):
                trace.finish()
        with FrameLog(self.path, validate_reported_frametime=False) as trace:
            trace.finish()
            metrics = summarize(trace.frames, 0, 3)
            self.assertEqual(metrics["frames"], 2)
            self.assertAlmostEqual(metrics["fps"], 2 / 3)
            self.assertEqual(metrics["trailing_gap_ms"], 1000)
            self.assertEqual(trace.reported_timing_mismatches, 3)

    def test_elapsed_only_still_rejects_duplicate_timestamps(self):
        self.path.write_text("fps,frametime,elapsed\n1,40000000,100\n1,40000000,100\n")
        with FrameLog(self.path, validate_reported_frametime=False) as trace:
            with self.assertRaisesRegex(ValueError, "strictly increasing"):
                trace.finish()

    def test_partial_live_rows_are_not_parsed_prematurely(self):
        self.path.write_text("os,cpu\nLinux,CPU\nfps,frametime,elapsed\n100,10,10000000\n100,10,")
        with FrameLog(self.path) as trace:
            trace.poll()
            self.assertEqual(len(trace.frames), 1)
            with self.path.open("a") as file:
                file.write("20000000\n")
            trace.poll()
            trace.poll()
            self.assertEqual(len(trace.frames), 2)
            self.assertEqual(trace.frames[-1].elapsed_ns, 20000000)
            trace.finish()

    def test_incomplete_offline_csv_is_rejected(self):
        self.path.write_text("fps,frametime,elapsed\n100,10,10000000\n100,10,")
        with FrameLog(self.path) as trace:
            with self.assertRaisesRegex(ValueError, "Incomplete final"):
                trace.finish()

    def test_blinc_style_clock_corruption_is_rejected(self):
        self.path.write_text(
            "fps,frametime,elapsed\n100,10,10000000\n100,10,20000000\n0.00004,26497000,1020000000\n"
        )
        with FrameLog(self.path) as trace:
            with self.assertRaisesRegex(ValueError, "disagrees with elapsed"):
                trace.finish()

    def test_actual_long_gaps_are_valid_when_the_clocks_agree(self):
        self.path.write_text("fps,frametime,elapsed\n100,10,10000000\n0.1,10000,10010000000\n")
        with FrameLog(self.path) as trace:
            trace.finish()
            self.assertEqual(len(trace.frames), 2)

    def test_import_start_uses_the_next_frame_boundary(self):
        self.path.write_text("fps,frametime,elapsed\n10,100,0\n10,100,100000000\n10,100,200000000\n10,100,300000000\n")
        metrics = parse_csv(self.path, 0.1, start=0.05)
        self.assertEqual(metrics["measurement_start_elapsed_ns"], 100000000)
        self.assertEqual(metrics["frames"], 1)

    def test_old_short_capture_cannot_be_normalized_to_a_full_window(self):
        self.path.write_text("fps,frametime,elapsed\n100,10,2960000000\n100,10,2970000000\n")
        with self.assertRaisesRegex(ValueError, "does not bracket"):
            parse_csv(self.path, 30)


if __name__ == "__main__":
    unittest.main()
