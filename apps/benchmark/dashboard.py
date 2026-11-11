import ctypes
import math
import statistics
import time

from benchmark.frames import summarize as summarize_window
from benchmark.scroll import WheelInput

MEASUREMENT = "mangohud-dashboard-v3"
RECORD_COUNT = 1000
PHASE_DURATION = 2.0
PHASES = ("typing", "refresh", "toggle", "scroll")


class DashboardInput(WheelInput):
    def __init__(self, window):
        super().__init__(window)
        self.window = window
        for name in ("xdo_send_keysequence_window", "xdo_enter_text_window"):
            function = getattr(self.library, name)
            function.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_char_p, ctypes.c_uint]
            function.restype = ctypes.c_int
        for name in ("xdo_mouse_down", "xdo_mouse_up"):
            function = getattr(self.library, name)
            function.argtypes = [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int]
            function.restype = ctypes.c_int
        if abs(self.size[0] / self.size[1] - 1000 / 700) > 0.01:
            self.close()
            raise RuntimeError(f"Dashboard requires a 1000x700 logical client area, got {self.size}")

    def move(self, horizontal, vertical):
        self.check(self.library.xdo_move_mouse_relative_to_window(
            self.connection, self.window, round(horizontal * self.size[0] / 1000),
            round(vertical * self.size[1] / 700)))
        self.xlib.XFlush(self.display)

    def click(self, horizontal, vertical):
        self.move(horizontal, vertical)
        self.check(self.library.xdo_mouse_down(self.connection, 0, 1))
        self.check(self.library.xdo_mouse_up(self.connection, 0, 1))
        self.xlib.XFlush(self.display)

    def keys(self, sequence):
        self.check(self.library.xdo_send_keysequence_window(
            self.connection, 0, sequence.encode(), 1))
        self.xlib.XFlush(self.display)

    def type_text(self, value):
        self.check(self.library.xdo_enter_text_window(
            self.connection, 0, value.encode(), 1))
        self.xlib.XFlush(self.display)

    def wheel(self, button):
        self.move(500, 400)
        self.check(self.library.xdo_click_window(self.connection, 0, button))
        self.xlib.XFlush(self.display)

    def perform(self, action):
        if action == "focus-input":
            self.click(200, 90)
        elif action == "select-text":
            self.keys("ctrl+a")
        elif action == "backspace":
            self.keys("BackSpace")
        elif action.startswith("type-"):
            self.type_text(action.removeprefix("type-"))
        elif action == "refresh":
            self.click(496, 90)
        elif action == "toggle":
            self.click(704, 90)
        elif action == "scroll-down":
            self.wheel(5)
        elif action == "scroll-up":
            self.wheel(4)
        else:
            raise ValueError(f"Unknown dashboard action: {action}")


def exercise(input, poll, rate):
    actions = []
    phases = []
    started = time.monotonic()
    for phase_index, phase in enumerate(PHASES):
        phase_start = started + phase_index * PHASE_DURATION
        phase_end = phase_start + PHASE_DURATION
        next_event = phase_start
        events_sent = 0
        while time.monotonic() < phase_end:
            poll()
            now = time.monotonic()
            if now >= phase_end:
                break
            if now >= next_event:
                if phase == "typing":
                    action = "type-a" if events_sent % 2 == 0 else "backspace"
                elif phase == "scroll":
                    action = "scroll-down" if now < phase_start + PHASE_DURATION / 2 else "scroll-up"
                else:
                    action = phase
                input.perform(action)
                actions.append({"action": action, "started_monotonic_s": now,
                                "finished_monotonic_s": time.monotonic()})
                events_sent += 1
                next_event = max(next_event + 1 / rate, time.monotonic())
            remaining = min(next_event, phase_end) - time.monotonic()
            if remaining > 0:
                time.sleep(min(0.001, remaining))
        phases.append({"phase": phase, "start_offset_s": phase_index * PHASE_DURATION,
                       "duration_s": PHASE_DURATION, "events_sent": events_sent})
    poll()
    return {"actions": actions, "phases": phases, "duration_s": PHASE_DURATION * len(PHASES),
            "actual_duration_s": time.monotonic() - started}


def summarize(frames, start_ns=None, duration=None):
    if (start_ns is None) != (duration is None):
        raise ValueError("Dashboard window requires both a start and duration")
    if start_ns is not None:
        window = summarize_window(frames, start_ns, duration)
        end_ns = window["measurement_end_elapsed_ns"]
        selected = [frame for frame in frames if start_ns < frame.elapsed_ns <= end_ns]
        per_frame = []
        previous_ns = start_ns
        for frame in selected:
            per_frame.append({"elapsed_ns": frame.elapsed_ns,
                              "present_interval_ms": (frame.elapsed_ns - previous_ns) / 1e6})
            previous_ns = frame.elapsed_ns
        interval_stats = window["frame_interval_ms"]
        return {
            "present_interval_ms": ({name: interval_stats[name] for name in ("median", "p95", "max")}
                                    if interval_stats else None),
            "worst_presentation_gap_ms": window["max_frame_gap_ms"],
            "trailing_gap_ms": window["trailing_gap_ms"],
            "measurement_start_elapsed_ns": start_ns,
            "measurement_end_elapsed_ns": end_ns,
            "measurement_duration_s": duration,
            "per_frame": per_frame,
            "timing_scope": "Worst presentation gap in a fully bracketed workload window, including phase transitions and trailing silence. The initial anchor excludes pre-workload idle. Not CPU or GPU render duration or input latency.",
        }
    if len(frames) < 2:
        raise ValueError("Dashboard capture requires at least two frame timestamps")
    per_frame = [{"elapsed_ns": current.elapsed_ns,
                  "present_interval_ms": (current.elapsed_ns - previous.elapsed_ns) / 1e6}
                 for previous, current in zip(frames, frames[1:])]
    intervals = sorted(frame["present_interval_ms"] for frame in per_frame)
    return {
        "present_interval_ms": {
            "median": statistics.median(intervals),
            "p95": intervals[math.ceil(len(intervals) * 0.95) - 1],
            "max": intervals[-1],
        },
        "first_sample_elapsed_ns": frames[0].elapsed_ns,
        "per_frame": per_frame,
        "timing_scope": "MangoHud presentation intervals from consecutive elapsed timestamps, including idle time, input spacing, synchronization and limiting. First sample is an anchor. Not CPU or GPU render duration or input latency.",
    }
