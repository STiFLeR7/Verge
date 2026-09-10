"""Run under xvfb-run; exercises the real Linux binary, no user desktop/accounts."""
import os
import json
import ctypes as C
from datetime import datetime, timezone
from pathlib import Path
import subprocess
import sys
import tempfile
import time

binary = str(Path(sys.argv[1]).resolve())
def xdo(*args):
    return subprocess.check_output(["xdotool", *map(str, args)], text=True).strip()
def geometry(window):
    return dict(line.split("=", 1) for line in xdo("getwindowgeometry", "--shell", window).splitlines())
def wait_for(predicate, timeout):
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        if predicate():
            return
        if process.poll() is not None:
            raise AssertionError("Verge exited: " + process.stderr.read())
        time.sleep(0.1)
    raise AssertionError("Timed out")

# Capture native X11 pixels directly; no screenshot package or renderer instrumentation.
xlib = C.CDLL("libX11.so.6")
xlib.XOpenDisplay.argtypes = [C.c_char_p]
xlib.XOpenDisplay.restype = C.c_void_p
xlib.XGetImage.argtypes = [C.c_void_p, C.c_ulong, C.c_int, C.c_int, C.c_uint, C.c_uint, C.c_ulong, C.c_int]
xlib.XGetImage.restype = C.c_void_p
xlib.XGetPixel.argtypes = [C.c_void_p, C.c_int, C.c_int]
xlib.XGetPixel.restype = C.c_ulong
xlib.XDestroyImage.argtypes = [C.c_void_p]
xlib.XCloseDisplay.argtypes = [C.c_void_p]
def pixels(window):
    display = xlib.XOpenDisplay(None)
    assert display, "Cannot open capture display"
    image = None
    try:
        image = xlib.XGetImage(display, int(window), 0, 0, 320, 272, C.c_ulong(-1), 2)
        assert image, "Cannot capture native window"
        return tuple(xlib.XGetPixel(image, x, y) for y in range(272) for x in range(320))
    finally:
        if image: xlib.XDestroyImage(image)
        xlib.XCloseDisplay(display)
def click_footer(window, x):
    xdo("mousemove", "--window", window, x, 256)
    xdo("click", 1)
    time.sleep(0.3)

with tempfile.TemporaryDirectory(prefix="verge-native-") as home:
    sessions = Path(home) / "sessions"
    sessions.mkdir()
    timestamp = datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")
    for name in ("first", "second"):
        records = [
            {"timestamp": timestamp, "type": "turn_context", "payload": {"model": "fixture-model", "cwd": "/fixture/" + name}},
            {"timestamp": timestamp, "type": "event_msg", "payload": {"type": "task_started"}},
        ]
        (sessions / (name + ".jsonl")).write_text("\n".join(map(json.dumps, records)) + "\n")
    env = dict(os.environ, HOME=home, CODEX_HOME=home, USERPROFILE=home, LOCALAPPDATA=home)
    if os.environ.get("VERGE_TEST_TCP") == "1":
        env["DISPLAY"] = "localhost" + os.environ["DISPLAY"]
    xdo("mousemove", 0, 0)
    focus = xdo("getwindowfocus", "-f")
    process = subprocess.Popen([binary], env=env, stderr=subprocess.PIPE, text=True)
    try:
        windows = []
        def find_window():
            found = subprocess.run(["xdotool", "search", "--name", "^Verge$"], capture_output=True, text=True)
            windows[:] = found.stdout.splitlines()
            return bool(windows)
        wait_for(find_window, 10)
        window = windows[0]
        assert geometry(window)["HEIGHT"] == "272"
        assert xdo("getwindowfocus", "-f") == focus, "Startup stole focus"
        time.sleep(0.5)  # Let the first off-thread metadata snapshot arrive.
        overview = pixels(window)
        click_footer(window, 20)
        first = pixels(window)
        assert first != overview, "Sessions did not open"
        click_footer(window, 300)
        second = pixels(window)
        assert first != second, "Next did not change rendered session"
        click_footer(window, 210)
        assert pixels(window) == second, "Counter navigated"
        click_footer(window, 120)
        assert pixels(window) == first, "Previous did not restore first session"
        click_footer(window, 120)
        assert pixels(window) == second, "Previous did not wrap"
        click_footer(window, 300)
        assert pixels(window) == first, "Next did not wrap"
        click_footer(window, 20)
        assert pixels(window) == overview, "Back did not restore overview"
        print("PASS: native mouse session navigation, counter, previous/next wrap, Back", flush=True)
        xdo("mousemove", 0, 0)
        wait_for(lambda: geometry(window)["HEIGHT"] == "6", 36)
        g = geometry(window)
        xdo("mousemove", int(g["X"]) + 10, int(g["Y"]) + 2)
        wait_for(lambda: geometry(window)["HEIGHT"] == "272", 3)
        xdo("click", 1)
        assert xdo("getwindowfocus", "-f") == focus, "Interaction stole focus"
        print("PASS: real X11 window, working-session inactivity collapse, hover reveal, no focus theft")
    finally:
        process.terminate()
        try:
            process.wait(timeout=3)
        except subprocess.TimeoutExpired:
            process.kill()
            process.wait()

