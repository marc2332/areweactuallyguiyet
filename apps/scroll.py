#!/usr/bin/env python3
import argparse
import contextlib
import ctypes
import math
import socket
import time

XRES_CLIENT_ID_PID_MASK = 2


class ClientIdSpec(ctypes.Structure):
    _fields_ = [("client", ctypes.c_ulong), ("mask", ctypes.c_uint)]


class ClientIdValue(ctypes.Structure):
    _fields_ = [("spec", ClientIdSpec), ("length", ctypes.c_long), ("value", ctypes.c_void_p)]


def window_client_pid(window):
    xlib = ctypes.CDLL("libX11.so.6")
    resource = ctypes.CDLL("libXRes.so.1")
    xlib.XOpenDisplay.argtypes = [ctypes.c_char_p]
    xlib.XOpenDisplay.restype = ctypes.c_void_p
    xlib.XCloseDisplay.argtypes = [ctypes.c_void_p]
    resource.XResQueryClientIds.argtypes = [
        ctypes.c_void_p, ctypes.c_long, ctypes.POINTER(ClientIdSpec),
        ctypes.POINTER(ctypes.c_long), ctypes.POINTER(ctypes.POINTER(ClientIdValue)),
    ]
    resource.XResGetClientPid.argtypes = [ctypes.POINTER(ClientIdValue)]
    resource.XResGetClientPid.restype = ctypes.c_int
    resource.XResClientIdsDestroy.argtypes = [ctypes.c_long, ctypes.POINTER(ClientIdValue)]
    resource.XResClientIdsDestroy.restype = None
    display = xlib.XOpenDisplay(None)
    if not display:
        raise RuntimeError("Cannot connect to the X11 display")
    count = ctypes.c_long()
    values = ctypes.POINTER(ClientIdValue)()
    try:
        spec = ClientIdSpec(window, XRES_CLIENT_ID_PID_MASK)
        resource.XResQueryClientIds(display, 1, ctypes.byref(spec), ctypes.byref(count), ctypes.byref(values))
        for index in range(count.value):
            process_id = resource.XResGetClientPid(ctypes.byref(values[index]))
            if process_id > 0:
                return process_id
        return None
    finally:
        if values:
            resource.XResClientIdsDestroy(count, values)
        xlib.XCloseDisplay(display)


class WheelInput:
    def __init__(self, window):
        self.xlib = ctypes.CDLL("libX11.so.6")
        self.xlib.XOpenDisplay.argtypes = [ctypes.c_char_p]
        self.xlib.XOpenDisplay.restype = ctypes.c_void_p
        self.xlib.XFlush.argtypes = [ctypes.c_void_p]
        self.xlib.XCloseDisplay.argtypes = [ctypes.c_void_p]
        self.library = ctypes.CDLL("libxdo.so.3")
        self.library.xdo_new_with_opened_display.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_int]
        self.library.xdo_new_with_opened_display.restype = ctypes.c_void_p
        self.library.xdo_free.argtypes = [ctypes.c_void_p]
        for name, types in {
            "xdo_activate_window": [ctypes.c_void_p, ctypes.c_ulong],
            "xdo_wait_for_window_active": [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int],
            "xdo_move_mouse_relative_to_window": [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int, ctypes.c_int],
            "xdo_get_window_size": [ctypes.c_void_p, ctypes.c_ulong, ctypes.POINTER(ctypes.c_uint), ctypes.POINTER(ctypes.c_uint)],
            "xdo_click_window": [ctypes.c_void_p, ctypes.c_ulong, ctypes.c_int],
        }.items():
            getattr(self.library, name).argtypes = types
            getattr(self.library, name).restype = ctypes.c_int
        self.display = self.xlib.XOpenDisplay(None)
        if not self.display:
            raise RuntimeError("Cannot connect to the X11 display")
        self.events_sent = 0
        self.connection = self.library.xdo_new_with_opened_display(self.display, None, 1)
        if not self.connection:
            self.xlib.XCloseDisplay(self.display)
            raise RuntimeError("Cannot initialize XTest input")
        try:
            width, height = ctypes.c_uint(), ctypes.c_uint()
            self.check(self.library.xdo_get_window_size(self.connection, window, ctypes.byref(width), ctypes.byref(height)))
            self.size = (width.value, height.value)
            deadline = time.monotonic() + 5
            while True:
                self.check(self.library.xdo_activate_window(self.connection, window))
                if self.library.xdo_wait_for_window_active(self.connection, window, 1) == 0:
                    break
                if time.monotonic() >= deadline:
                    raise RuntimeError("Cannot activate the benchmark window")
            self.check(self.library.xdo_move_mouse_relative_to_window(self.connection, window, width.value // 2 - 1, height.value // 2))
            self.xlib.XFlush(self.display)
            time.sleep(0.05)
            self.check(self.library.xdo_move_mouse_relative_to_window(self.connection, window, width.value // 2, height.value // 2))
            self.xlib.XFlush(self.display)
        except BaseException:
            self.close()
            raise

    @staticmethod
    def check(result):
        if result != 0:
            raise RuntimeError("XTest input operation failed")

    def scroll(self, duration, rate, tick=None):
        start = time.monotonic()
        count = 0
        next_event = start
        while time.monotonic() - start < duration:
            if tick and tick() is False:
                break
            now = time.monotonic()
            if now >= next_event:
                button = 5 if int((now - start) / 4) % 2 == 0 else 4
                self.check(self.library.xdo_click_window(self.connection, 0, button))
                self.xlib.XFlush(self.display)
                self.events_sent += 1
                count += 1
                next_event += 1 / rate
                if next_event < now:
                    next_event = now + 1 / rate
            time.sleep(max(0, min(0.01, next_event - time.monotonic(), start + duration - time.monotonic())))
        return count

    def close(self):
        if self.connection:
            self.library.xdo_free(self.connection)
            self.connection = None

    def __enter__(self):
        return self

    def __exit__(self, *_):
        self.close()


def positive_number(value):
    number = float(value)
    if not math.isfinite(number) or number <= 0:
        raise argparse.ArgumentTypeError("must be finite and positive")
    return number


def main():
    parser = argparse.ArgumentParser(description="Send external XTest wheel events and control MangoHud logging.")
    parser.add_argument("window", type=lambda value: int(value, 0), help="X11 window ID from xwininfo")
    parser.add_argument("--duration", type=positive_number, default=15)
    parser.add_argument("--rate", type=positive_number, default=144, help="Wheel notches per second")
    parser.add_argument("--control", default="freya-scroll", help="MangoHud abstract control socket name")
    arguments = parser.parse_args()
    with WheelInput(arguments.window) as wheel:
        time.sleep(0.5)
        with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as control:
            control.connect("\0" + arguments.control)
            wheel.scroll(2, arguments.rate)
            control.sendall(b":logging=1;")
            try:
                count = wheel.scroll(arguments.duration, arguments.rate)
            except BaseException:
                with contextlib.suppress(OSError):
                    control.sendall(b":logging=0;")
                raise
            control.sendall(b":logging=0;")
            wheel.scroll(0.5, arguments.rate)
            print(f"Sent {count} wheel notches during the requested {arguments.duration:g}s capture.")


if __name__ == "__main__":
    main()
