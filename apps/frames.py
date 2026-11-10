import csv
from dataclasses import dataclass
import math
from pathlib import Path
import statistics


@dataclass(frozen=True)
class Frame:
    elapsed_ns: int
    reported_interval_ms: float


class FrameLog:
    def __init__(self, path, *, validate_reported_frametime=True):
        self.path = Path(path)
        self.validate_reported_frametime = validate_reported_frametime
        self.reported_timing_mismatches = 0
        self.file = None
        self.pending = ""
        self.header = None
        self.frames = []

    def poll(self):
        if self.file is None:
            self.file = self.path.open(newline="")
        lines = (self.pending + self.file.read()).split("\n")
        self.pending = lines.pop()
        for line in lines:
            if not line.strip():
                continue
            try:
                values = next(csv.reader([line]))
            except csv.Error as error:
                raise ValueError(f"Malformed CSV in {self.path}") from error
            if self.header is None:
                if {"fps", "frametime", "elapsed"}.issubset(values):
                    if len(set(values)) != len(values):
                        raise ValueError("Duplicate MangoHud header columns")
                    self.header = values
                continue
            if len(values) != len(self.header):
                raise ValueError(f"Malformed frame row in {self.path}")
            row = dict(zip(self.header, values))
            interval = float(row["frametime"])
            fps = float(row["fps"])
            elapsed = int(row["elapsed"])
            if not all(math.isfinite(value) and value > 0 for value in (interval, fps)) or elapsed < 0:
                raise ValueError(f"Invalid frame timing in {self.path}")
            if self.frames:
                gap_ms = (elapsed - self.frames[-1].elapsed_ns) / 1e6
                if gap_ms <= 0:
                    raise ValueError(f"Frame elapsed timestamps are not strictly increasing in {self.path}")
                if abs(gap_ms - interval) > max(1.0, gap_ms * 0.1):
                    self.reported_timing_mismatches += 1
                    if self.validate_reported_frametime:
                        raise ValueError(
                            f"MangoHud frametime disagrees with elapsed timestamps in {self.path}: "
                            f"{interval:g} ms reported, {gap_ms:g} ms elapsed"
                        )
            self.frames.append(Frame(elapsed, interval))

    def finish(self):
        self.poll()
        if self.pending.strip():
            raise ValueError(f"Incomplete final frame row in {self.path}")
        if self.header is None:
            raise ValueError(f"No MangoHud frame table in {self.path}")
        if not self.frames:
            raise ValueError(f"No frames in {self.path}")

    def close(self):
        if self.file:
            self.file.close()

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def duration_ns(duration):
    if not math.isfinite(duration) or duration <= 0:
        raise ValueError("Capture duration must be finite and positive")
    if not math.isfinite(duration * 1e9):
        raise ValueError("Capture duration is too large")
    nanoseconds = round(duration * 1e9)
    if nanoseconds < 1:
        raise ValueError("Capture duration must be at least one nanosecond")
    return nanoseconds


def summarize(frames, start_ns, duration):
    end_ns = start_ns + duration_ns(duration)
    anchors = [frame for frame in frames if frame.elapsed_ns == start_ns]
    if not anchors or not frames or frames[-1].elapsed_ns < end_ns:
        raise ValueError("CSV does not bracket the complete measurement window")
    selected = [frame for frame in frames if start_ns < frame.elapsed_ns <= end_ns]
    previous_ns = start_ns
    intervals = []
    for frame in selected:
        intervals.append((frame.elapsed_ns - previous_ns) / 1e6)
        previous_ns = frame.elapsed_ns
    trailing_gap_ms = (end_ns - previous_ns) / 1e6
    intervals.sort()
    interval_stats = None
    if intervals:
        interval_stats = {
            "median": statistics.median(intervals),
            "p95": intervals[math.ceil(len(intervals) * 0.95) - 1],
            "p99": intervals[math.ceil(len(intervals) * 0.99) - 1],
            "max": intervals[-1],
        }
    return {
        "frames": len(selected),
        "fps": len(selected) * 1e9 / (end_ns - start_ns),
        "measurement_duration_s": (end_ns - start_ns) / 1e9,
        "measurement_start_elapsed_ns": start_ns,
        "measurement_end_elapsed_ns": end_ns,
        "raw_frames": len(frames),
        "first_sample_elapsed_s": frames[0].elapsed_ns / 1e9,
        "last_sample_elapsed_s": frames[-1].elapsed_ns / 1e9,
        "frame_interval_ms": interval_stats,
        "trailing_gap_ms": trailing_gap_ms,
        "max_frame_gap_ms": max([trailing_gap_ms, *intervals]),
    }


def parse_csv(path, duration, start=None):
    duration_ns(duration)
    if start is not None and (not math.isfinite(start) or not math.isfinite(start * 1e9) or start < 0):
        raise ValueError("Start elapsed time must be finite and nonnegative")
    with FrameLog(path) as trace:
        trace.finish()
        requested_start = trace.frames[0].elapsed_ns if start is None else round(start * 1e9)
        anchor = next((frame for frame in trace.frames if frame.elapsed_ns >= requested_start), None)
        if anchor is None:
            raise ValueError("No frame boundary at or after the requested start")
        return summarize(trace.frames, anchor.elapsed_ns, duration)


class CaptureWindow:
    def __init__(self, duration, warmup, timeout, scrolling_started):
        self.duration_ns = duration_ns(duration)
        self.warmup_end = scrolling_started + warmup
        self.ready_deadline = self.warmup_end + timeout
        self.capture_timeout = duration + timeout
        self.start_ns = None
        self.start_wall = None
        self.start_events = None
        self.end_events = None
        self.end_wall = None
        self.seen_frames = 0

    def update(self, frames, now, events_sent):
        fresh = len(frames) > self.seen_frames
        self.seen_frames = len(frames)
        if self.start_ns is None:
            if now > self.ready_deadline:
                raise RuntimeError("No fresh validated frame boundary after scrolling warmup")
            if now >= self.warmup_end and fresh and len(frames) >= 2:
                self.start_ns = frames[-1].elapsed_ns
                self.start_wall = now
                self.start_events = events_sent
        elif frames[-1].elapsed_ns >= self.start_ns + self.duration_ns:
            self.end_events = events_sent
            self.end_wall = now
            return False
        elif now - self.start_wall > self.capture_timeout:
            raise RuntimeError("Renderer did not bracket the measurement endpoint while scrolling")
        return True
