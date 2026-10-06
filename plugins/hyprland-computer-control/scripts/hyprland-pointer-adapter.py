#!/usr/bin/env python3
"""Short-lived, session-local guard for a bounded Wayland pointer helper."""

import argparse
from decimal import Decimal, InvalidOperation
import hashlib
import json
import os
from pathlib import Path
import secrets
import select
import shutil
import signal
import stat
import subprocess
import sys
import time


SCHEMA_VERSION = 1
MAX_JSON_BYTES = 64 * 1024
MAX_SCREENSHOT_BYTES = 128 * 1024 * 1024
MAX_ACTIONS = 64
MAX_OBSERVATION_AGE_MS = 30_000
MAX_OPERATION_SECONDS = 12
HELPER_REPLY_SECONDS = 3
BUTTONS = {"left": 0x110, "right": 0x111, "middle": 0x112}
SAFE_COMPONENT_CHARS = frozenset("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789_-.")


class AdapterError(Exception):
    pass


def safe_component(value):
    return (
        isinstance(value, str)
        and 1 <= len(value) <= 256
        and value[0] not in ".-"
        and all(character in SAFE_COMPONENT_CHARS for character in value)
    )


def owned_path(path, kind, uid, private=False):
    try:
        info = path.lstat()
    except OSError:
        return None
    if info.st_uid != uid or stat.S_ISLNK(info.st_mode):
        return None
    if kind == "directory" and not stat.S_ISDIR(info.st_mode):
        return None
    if kind == "socket" and not stat.S_ISSOCK(info.st_mode):
        return None
    if kind == "file" and not stat.S_ISREG(info.st_mode):
        return None
    if private and info.st_mode & 0o077:
        return None
    return info


def private_real_parent(path, uid):
    try:
        parent = path.parent.resolve(strict=True)
    except OSError:
        return False
    return parent == path.parent and owned_path(parent, "directory", uid, private=True) is not None


def session_snapshot(env=None, uid=None):
    env = os.environ if env is None else env
    uid = os.getuid() if uid is None else uid
    runtime = Path(env.get("XDG_RUNTIME_DIR", ""))
    display = env.get("WAYLAND_DISPLAY", "")
    instance = env.get("HYPRLAND_INSTANCE_SIGNATURE", "")
    if env.get("XDG_SESSION_TYPE") != "wayland":
        raise AdapterError("not an inherited Wayland session")
    runtime_info = owned_path(runtime, "directory", uid, private=True) if runtime.is_absolute() else None
    if runtime_info is None:
        raise AdapterError("unsafe runtime directory")
    try:
        canonical_runtime = runtime.resolve(strict=True)
    except OSError as error:
        raise AdapterError("unsafe runtime directory") from error
    wayland_info = owned_path(runtime / display, "socket", uid) if safe_component(display) else None
    if wayland_info is None:
        raise AdapterError("unsafe Wayland socket")
    hypr_socket = runtime / "hypr" / instance / ".socket.sock"
    hypr_info = owned_path(hypr_socket, "socket", uid) if safe_component(instance) else None
    if hypr_info is None:
        raise AdapterError("unsafe Hyprland socket")
    return {
        "uid": uid,
        "runtime_dir": {"path": str(canonical_runtime), "device": runtime_info.st_dev, "inode": runtime_info.st_ino},
        "wayland_display": display,
        "wayland_socket": {"device": wayland_info.st_dev, "inode": wayland_info.st_ino},
        "hyprland_instance_signature": instance,
        "hyprland_socket": {"device": hypr_info.st_dev, "inode": hypr_info.st_ino},
    }


def read_owned_file(path_string, uid, limit, private=True):
    path = Path(path_string)
    if not path.is_absolute() or not private_real_parent(path, uid):
        raise AdapterError("file path must be absolute")
    try:
        # Nonblocking open lets fstat reject FIFOs before waiting for a writer.
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        with os.fdopen(descriptor, "rb") as stream:
            info = os.fstat(stream.fileno())
            if info.st_uid != uid or not stat.S_ISREG(info.st_mode):
                raise AdapterError("unsafe or oversized file")
            if private and info.st_mode & 0o077:
                raise AdapterError("unsafe or oversized file")
            if info.st_size > limit:
                raise AdapterError("unsafe or oversized file")
            data = stream.read(limit + 1)
    except OSError as error:
        raise AdapterError("could not read file") from error
    if len(data) > limit:
        raise AdapterError("oversized file")
    return path, info, data


def stat_owned_file(path_string, uid, limit, private=True):
    path = Path(path_string)
    if not path.is_absolute() or not private_real_parent(path, uid):
        raise AdapterError("file path must be absolute")
    try:
        descriptor = os.open(path, os.O_RDONLY | os.O_NOFOLLOW | os.O_NONBLOCK)
        try:
            info = os.fstat(descriptor)
        finally:
            os.close(descriptor)
    except OSError as error:
        raise AdapterError("could not read file") from error
    if info.st_uid != uid or not stat.S_ISREG(info.st_mode) or info.st_size > limit:
        raise AdapterError("unsafe or oversized file")
    if private and info.st_mode & 0o077:
        raise AdapterError("unsafe or oversized file")
    return path, info


def read_json(path_string, uid):
    _, _, data = read_owned_file(path_string, uid, MAX_JSON_BYTES)
    try:
        value = json.loads(data.decode("utf-8"))
    except (UnicodeError, ValueError) as error:
        raise AdapterError("invalid JSON file") from error
    if not isinstance(value, dict):
        raise AdapterError("JSON root must be an object")
    return value


def query_json(hyprctl, query, env=None, timeout=2):
    try:
        result = subprocess.run(
            [hyprctl, "-j", query],
            env=env,
            stdin=subprocess.DEVNULL,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            text=True,
            encoding="utf-8",
            errors="strict",
            timeout=timeout,
            check=False,
        )
    except (OSError, UnicodeError, subprocess.TimeoutExpired) as error:
        raise AdapterError("Hyprland state query failed") from error
    if result.returncode != 0 or len(result.stdout) > MAX_JSON_BYTES:
        raise AdapterError("Hyprland state query failed")
    try:
        return json.loads(result.stdout)
    except ValueError as error:
        raise AdapterError("Hyprland returned invalid state") from error


def integer(value, field, minimum=None, maximum=None):
    if type(value) is not int:
        raise AdapterError(f"{field} must be an integer")
    if minimum is not None and value < minimum:
        raise AdapterError(f"{field} is out of range")
    if maximum is not None and value > maximum:
        raise AdapterError(f"{field} is out of range")
    return value


def canonical_scale(value):
    if isinstance(value, bool) or not isinstance(value, (int, float, str)):
        raise AdapterError("invalid monitor scale")
    try:
        scale = Decimal(str(value))
    except InvalidOperation as error:
        raise AdapterError("invalid monitor scale") from error
    normalized = format(scale.normalize(), "f")
    if not scale.is_finite() or normalized not in ("1", "2"):
        raise AdapterError("only monitor scales 1 and 2 are supported")
    return normalized


def canonical_monitors(value):
    if not isinstance(value, list) or not value or len(value) > 16:
        raise AdapterError("invalid monitor layout")
    monitors = []
    names = set()
    for monitor in value:
        if not isinstance(monitor, dict) or not safe_component(monitor.get("name")):
            raise AdapterError("invalid monitor")
        name = monitor["name"]
        if name in names:
            raise AdapterError("duplicate monitor")
        names.add(name)
        transform = integer(monitor.get("transform"), "monitor transform", 0, 7)
        if transform != 0:
            raise AdapterError("transformed monitors are unsupported")
        width = integer(monitor.get("width"), "monitor width", 1, 32768)
        height = integer(monitor.get("height"), "monitor height", 1, 32768)
        scale = canonical_scale(monitor.get("scale"))
        monitors.append(
            {
                "id": integer(monitor.get("id"), "monitor id", 0, 1024),
                "name": name,
                "x": integer(monitor.get("x"), "monitor x", -131072, 131072),
                "y": integer(monitor.get("y"), "monitor y", -131072, 131072),
                "width": width,
                "height": height,
                "scale": scale,
                "transform": transform,
            }
        )
    return sorted(monitors, key=lambda monitor: monitor["name"])


def active_window(value):
    if not isinstance(value, dict):
        raise AdapterError("invalid active-window state")
    address = value.get("address")
    if not isinstance(address, str) or not address.startswith("0x") or len(address) > 32:
        raise AdapterError("no unambiguous active window")
    try:
        int(address[2:], 16)
    except ValueError as error:
        raise AdapterError("invalid active-window address") from error
    at = value.get("at")
    size = value.get("size")
    monitor = value.get("monitor")
    if not isinstance(at, list) or len(at) != 2 or not isinstance(size, list) or len(size) != 2:
        raise AdapterError("invalid active-window geometry")
    return {
        "address": address.lower(),
        "monitor": integer(monitor, "window monitor", 0, 1024),
        "x": integer(at[0], "window x", -131072, 131072),
        "y": integer(at[1], "window y", -131072, 131072),
        "width": integer(size[0], "window width", 1, 32768),
        "height": integer(size[1], "window height", 1, 32768),
    }


def live_state(hyprctl, env=None):
    locked = query_json(hyprctl, "locked", env)
    if not isinstance(locked, dict) or type(locked.get("locked")) is not bool:
        raise AdapterError("unknown lock state")
    if locked["locked"]:
        raise AdapterError("session is locked")
    return {
        "focused_window": active_window(query_json(hyprctl, "activewindow", env)),
        "monitors": canonical_monitors(query_json(hyprctl, "monitors", env)),
    }


def png_dimensions(data):
    if len(data) < 24 or data[:8] != b"\x89PNG\r\n\x1a\n" or data[12:16] != b"IHDR":
        raise AdapterError("screenshot is not a PNG")
    width = int.from_bytes(data[16:20], "big")
    height = int.from_bytes(data[20:24], "big")
    if not (1 <= width <= 32768 and 1 <= height <= 32768):
        raise AdapterError("invalid screenshot dimensions")
    return width, height


def atomic_private_json(path_string, value, uid):
    path = Path(path_string)
    if not path.is_absolute() or path.parent == path or not private_real_parent(path, uid):
        raise AdapterError("output path must be absolute")
    payload = (json.dumps(value, sort_keys=True, separators=(",", ":")) + "\n").encode()
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        with os.fdopen(descriptor, "wb") as stream:
            stream.write(payload)
    except OSError as error:
        raise AdapterError("could not create observation file") from error


def target_monitor(state):
    window = state["focused_window"]
    matching = [monitor for monitor in state["monitors"] if monitor["id"] == window["monitor"]]
    if len(matching) != 1:
        raise AdapterError("target window monitor is unavailable")
    monitor = matching[0]
    scale = Decimal(monitor["scale"])
    right = Decimal(monitor["x"]) + Decimal(monitor["width"]) / scale
    bottom = Decimal(monitor["y"]) + Decimal(monitor["height"]) / scale
    if not (
        Decimal(monitor["x"]) <= window["x"]
        and Decimal(monitor["y"]) <= window["y"]
        and Decimal(window["x"] + window["width"]) <= right
        and Decimal(window["y"] + window["height"]) <= bottom
    ):
        raise AdapterError("target window crosses a monitor boundary")
    return monitor


def capture_screenshot(grim, screenshot_path, window, env, uid, scale="1"):
    path = Path(screenshot_path)
    if not path.is_absolute() or path.parent == path or not private_real_parent(path, uid):
        raise AdapterError("screenshot output path must be absolute")
    created = False
    try:
        descriptor = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW, 0o600)
        created = True
        os.close(descriptor)
        geometry = f"{window['x']},{window['y']} {window['width']}x{window['height']}"
        result = subprocess.run(
            [grim, "-s", canonical_scale(scale), "-g", geometry, str(path)], env=env, stdin=subprocess.DEVNULL,
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=3, check=False,
        )
        if result.returncode != 0:
            raise AdapterError("screenshot capture failed")
        info = owned_path(path, "file", uid, private=True)
        if info is None or info.st_size > MAX_SCREENSHOT_BYTES:
            raise AdapterError("unsafe or oversized screenshot")
        return path
    except BaseException as error:
        if created:
            try:
                path.unlink()
            except OSError:
                pass
        if not isinstance(error, Exception) or isinstance(error, AdapterError):
            raise
        raise AdapterError("screenshot capture failed") from error


def create_observation(screenshot_path, output_path, target_address, env=None, uid=None, now=None, monotonic=None, which=shutil.which):
    env = dict(os.environ if env is None else env)
    uid = os.getuid() if uid is None else uid
    session = session_snapshot(env, uid)
    hyprctl = which("hyprctl")
    grim = which("grim")
    if not hyprctl or not grim:
        raise AdapterError("required session client is unavailable")
    before = live_state(hyprctl, env)
    if active_window({"address": target_address, "monitor": before["focused_window"]["monitor"],
                      "at": [before["focused_window"]["x"], before["focused_window"]["y"]],
                      "size": [before["focused_window"]["width"], before["focused_window"]["height"]]})["address"] != before["focused_window"]["address"]:
        raise AdapterError("requested target window is not focused")
    monitor = target_monitor(before)
    # Start the observation clock before capture so capture and all subsequent
    # validation consume the same bounded freshness budget.
    captured_at_unix_ms = int((time.time() if now is None else now()) * 1000)
    captured_at_monotonic_ms = int((time.monotonic() if monotonic is None else monotonic()) * 1000)
    screenshot = capture_screenshot(grim, screenshot_path, before["focused_window"], env, uid, monitor["scale"])
    try:
        if session_snapshot(env, uid) != session:
            raise AdapterError("graphical session changed during capture")
        after = live_state(hyprctl, env)
        if after != before:
            raise AdapterError("target changed during capture")
        _, info, data = read_owned_file(str(screenshot), uid, MAX_SCREENSHOT_BYTES)
        width, height = png_dimensions(data)
        expected_width = before["focused_window"]["width"] * int(monitor["scale"])
        expected_height = before["focused_window"]["height"] * int(monitor["scale"])
        if width != expected_width or height != expected_height:
            raise AdapterError("screenshot dimensions do not match the target window")
        observation = {
            "schema_version": SCHEMA_VERSION,
            "observation_id": secrets.token_hex(16),
            "observed_at_unix_ms": captured_at_unix_ms,
            "observed_at_monotonic_ms": captured_at_monotonic_ms,
            "session": session,
            "focused_window": before["focused_window"],
            "monitors": before["monitors"],
            "target_monitor": monitor["name"],
            "screenshot": {
                "path": str(screenshot),
                "width": width,
                "height": height,
                "sha256": hashlib.sha256(data).hexdigest(),
                "device": info.st_dev,
                "inode": info.st_ino,
                "size": info.st_size,
                "mtime_ns": info.st_mtime_ns,
            },
        }
        atomic_private_json(output_path, observation, uid)
    except BaseException:
        screenshot.unlink(missing_ok=True)
        raise
    return observation


def validate_observation(value):
    required = {
        "schema_version", "observation_id", "observed_at_unix_ms",
        "observed_at_monotonic_ms", "session", "focused_window", "monitors",
        "target_monitor", "screenshot",
    }
    if set(value) != required or value.get("schema_version") != SCHEMA_VERSION:
        raise AdapterError("unsupported observation schema")
    if not isinstance(value["observation_id"], str) or len(value["observation_id"]) != 32:
        raise AdapterError("invalid observation identifier")
    try:
        int(value["observation_id"], 16)
    except ValueError as error:
        raise AdapterError("invalid observation identifier") from error
    integer(value["observed_at_unix_ms"], "observation time", 0)
    integer(value["observed_at_monotonic_ms"], "observation time", 0)
    if not isinstance(value["session"], dict) or set(value["session"]) != {
        "uid", "runtime_dir", "wayland_display", "wayland_socket",
        "hyprland_instance_signature", "hyprland_socket"
    }:
        raise AdapterError("invalid observation session")
    integer(value["session"]["uid"], "session uid", 0)
    if not safe_component(value["session"]["wayland_display"]) or not safe_component(value["session"]["hyprland_instance_signature"]):
        raise AdapterError("invalid observation session")
    runtime_dir = value["session"]["runtime_dir"]
    if not isinstance(runtime_dir, dict) or set(runtime_dir) != {"path", "device", "inode"}:
        raise AdapterError("invalid observation session")
    runtime_path = runtime_dir["path"]
    if not isinstance(runtime_path, str) or not Path(runtime_path).is_absolute() or os.path.normpath(runtime_path) != runtime_path:
        raise AdapterError("invalid observation session")
    for identity in (runtime_dir, value["session"]["wayland_socket"], value["session"]["hyprland_socket"]):
        if not isinstance(identity, dict) or "device" not in identity or "inode" not in identity:
            raise AdapterError("invalid observation session")
        if identity is not runtime_dir and set(identity) != {"device", "inode"}:
            raise AdapterError("invalid observation session")
        integer(identity["device"], "session device", 0)
        integer(identity["inode"], "session inode", 0)
    window = value["focused_window"]
    if not isinstance(window, dict) or set(window) != {"address", "monitor", "x", "y", "width", "height"}:
        raise AdapterError("invalid observed window")
    value["focused_window"] = active_window({
        "address": window["address"], "monitor": window["monitor"],
        "at": [window["x"], window["y"]], "size": [window["width"], window["height"]],
    })
    value["monitors"] = canonical_monitors(value["monitors"])
    if not safe_component(value["target_monitor"]):
        raise AdapterError("invalid target monitor")
    matching = [item for item in value["monitors"] if item["name"] == value["target_monitor"]]
    if len(matching) != 1:
        raise AdapterError("invalid target monitor")
    screenshot = value["screenshot"]
    if not isinstance(screenshot, dict) or set(screenshot) != {
        "path", "width", "height", "sha256", "device", "inode", "size", "mtime_ns"
    }:
        raise AdapterError("invalid screenshot metadata")
    for field in ("width", "height", "device", "inode", "size", "mtime_ns"):
        integer(screenshot[field], f"screenshot {field}", 0)
    if not isinstance(screenshot["sha256"], str) or len(screenshot["sha256"]) != 64:
        raise AdapterError("invalid screenshot digest")
    try:
        int(screenshot["sha256"], 16)
    except ValueError as error:
        raise AdapterError("invalid screenshot digest") from error
    observed_state = {"focused_window": value["focused_window"], "monitors": value["monitors"]}
    monitor = target_monitor(observed_state)
    if monitor["name"] != value["target_monitor"]:
        raise AdapterError("window monitor mismatch")
    if screenshot["width"] != window["width"] * int(monitor["scale"]) or screenshot["height"] != window["height"] * int(monitor["scale"]):
        raise AdapterError("screenshot geometry mismatch")
    return value


def validate_actions(value, observation_id):
    if set(value) != {"schema_version", "observation_id", "actions"} or value.get("schema_version") != SCHEMA_VERSION:
        raise AdapterError("unsupported action schema")
    if value.get("observation_id") != observation_id:
        raise AdapterError("actions refer to another observation")
    actions = value.get("actions")
    if not isinstance(actions, list) or not 1 <= len(actions) <= MAX_ACTIONS:
        raise AdapterError("invalid action count")
    held = []
    checked = []
    for action in actions:
        if not isinstance(action, dict) or not isinstance(action.get("type"), str):
            raise AdapterError("invalid action")
        kind = action["type"]
        if kind == "move":
            if set(action) != {"type", "x", "y"}:
                raise AdapterError("invalid move action")
            checked.append({"type": kind, "x": integer(action["x"], "x", 0), "y": integer(action["y"], "y", 0)})
        elif kind == "button":
            if set(action) != {"type", "button", "state", "x", "y"} or action.get("button") not in BUTTONS or action.get("state") not in ("down", "up"):
                raise AdapterError("invalid button action")
            button = action["button"]
            if action["state"] == "down":
                if button in held:
                    raise AdapterError("button is already held")
                held.append(button)
            else:
                if button not in held:
                    raise AdapterError("button is not held")
                held.remove(button)
            checked.append(dict(action, x=integer(action["x"], "x", 0), y=integer(action["y"], "y", 0)))
        elif kind == "scroll":
            if set(action) != {"type", "dx", "dy", "x", "y"}:
                raise AdapterError("invalid scroll action")
            dx = integer(action["dx"], "scroll dx", -20, 20)
            dy = integer(action["dy"], "scroll dy", -20, 20)
            if dx == dy == 0:
                raise AdapterError("empty scroll action")
            checked.append({"type": kind, "dx": dx, "dy": dy,
                            "x": integer(action["x"], "x", 0), "y": integer(action["y"], "y", 0)})
        else:
            raise AdapterError("unsupported action")
    if held:
        raise AdapterError("action sequence leaves a button held")
    return checked


def observation_is_fresh(observation, now=None, monotonic=None):
    unix_ms = int((time.time() if now is None else now()) * 1000)
    monotonic_ms = int((time.monotonic() if monotonic is None else monotonic()) * 1000)
    wall_age = unix_ms - observation["observed_at_unix_ms"]
    monotonic_age = monotonic_ms - observation["observed_at_monotonic_ms"]
    return -1_000 <= wall_age <= MAX_OBSERVATION_AGE_MS and 0 <= monotonic_age <= MAX_OBSERVATION_AGE_MS


def verify_screenshot(observation, uid, hash_contents=False):
    screenshot = observation["screenshot"]
    if hash_contents:
        path, info, data = read_owned_file(screenshot["path"], uid, MAX_SCREENSHOT_BYTES)
    else:
        path, info = stat_owned_file(screenshot["path"], uid, MAX_SCREENSHOT_BYTES)
        data = None
    actual = (info.st_dev, info.st_ino, info.st_size, info.st_mtime_ns)
    expected = (screenshot["device"], screenshot["inode"], screenshot["size"], screenshot["mtime_ns"])
    if actual != expected:
        raise AdapterError("screenshot changed after observation")
    if hash_contents and hashlib.sha256(data).hexdigest() != screenshot["sha256"]:
        raise AdapterError("screenshot changed after observation")
    return path


def assert_live_target(observation, env, uid, hyprctl, now=None, monotonic=None):
    if session_snapshot(env, uid) != observation["session"]:
        raise AdapterError("graphical session changed")
    if not observation_is_fresh(observation, now, monotonic):
        raise AdapterError("observation is stale")
    verify_screenshot(observation, uid, hash_contents=False)
    state = live_state(hyprctl, env)
    if state["focused_window"] != observation["focused_window"]:
        raise AdapterError("focused window or geometry changed")
    if state["monitors"] != observation["monitors"]:
        raise AdapterError("monitor layout changed")
    # Queries are bounded individually but may still consume the remainder of
    # the observation lifetime. Recheck both identity and freshness afterward.
    if session_snapshot(env, uid) != observation["session"]:
        raise AdapterError("graphical session changed")
    if not observation_is_fresh(observation, now, monotonic):
        raise AdapterError("observation is stale")


def move_command(action, observation):
    target = next(item for item in observation["monitors"] if item["name"] == observation["target_monitor"])
    screenshot = observation["screenshot"]
    if action["x"] >= screenshot["width"] or action["y"] >= screenshot["height"]:
        raise AdapterError("pointer coordinate is outside the screenshot")
    scale = Decimal(target["scale"])
    window = observation["focused_window"]
    logical_x = Decimal(window["x"]) + Decimal(action["x"]) / scale
    logical_y = Decimal(window["y"]) + Decimal(action["y"]) / scale
    left = min(Decimal(item["x"]) for item in observation["monitors"])
    top = min(Decimal(item["y"]) for item in observation["monitors"])
    right = max(Decimal(item["x"]) + Decimal(item["width"]) / Decimal(item["scale"]) for item in observation["monitors"])
    bottom = max(Decimal(item["y"]) + Decimal(item["height"]) / Decimal(item["scale"]) for item in observation["monitors"])
    # Half-logical units exactly preserve scale-2 screenshot pixel positions and
    # negative monitor offsets without floating-point parsing in the helper.
    unit = Decimal(2)
    x = int((logical_x - left) * unit)
    y = int((logical_y - top) * unit)
    x_extent = int((right - left) * unit)
    y_extent = int((bottom - top) * unit)
    if not (0 <= x <= x_extent and 0 <= y <= y_extent):
        raise AdapterError("mapped pointer coordinate is outside the layout")
    return f"move {x} {y} {x_extent} {y_extent}"


def helper_command(action, observation):
    position = move_command(action, observation)
    if action["type"] == "move":
        return position
    if action["type"] == "button":
        return f"button {position[5:]} {BUTTONS[action['button']]} {1 if action['state'] == 'down' else 0}"
    return f"scroll {position[5:]} {action['dx']} {action['dy']}"


class HelperProcess:
    def __init__(self, executable, env, deadline):
        self.deadline = deadline
        self.cancelled = False
        try:
            self.process = subprocess.Popen(
                [executable], env=env, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                stderr=subprocess.DEVNULL, text=True, encoding="ascii", bufsize=1,
            )
        except OSError as error:
            raise AdapterError("pointer helper could not start") from error
        self.previous_handlers = {}
        for signum in (signal.SIGINT, signal.SIGTERM, signal.SIGHUP):
            self.previous_handlers[signum] = signal.signal(signum, self._forward_signal)

    def _forward_signal(self, signum, _frame):
        self.cancelled = True
        if self.process.poll() is None:
            self.process.send_signal(signum)

    def exchange(self, command, expected):
        if self.cancelled:
            raise AdapterError("operation cancelled")
        if self.process.stdin is None or self.process.stdout is None:
            raise AdapterError("pointer helper pipe is unavailable")
        try:
            remaining = min(HELPER_REPLY_SECONDS, self.deadline - time.monotonic())
            if remaining <= 0:
                raise AdapterError("pointer helper timed out")
            if command is not None:
                if self.cancelled:
                    raise AdapterError("operation cancelled")
                self.process.stdin.write(command + "\n")
                self.process.stdin.flush()
            if self.cancelled:
                raise AdapterError("operation cancelled")
            remaining = min(HELPER_REPLY_SECONDS, self.deadline - time.monotonic())
            if remaining <= 0 or not select.select([self.process.stdout], [], [], remaining)[0]:
                raise AdapterError("pointer helper timed out")
            reply = self.process.stdout.readline()
        except (BrokenPipeError, OSError) as error:
            raise AdapterError("pointer helper disconnected") from error
        if reply != expected + "\n":
            raise AdapterError("pointer helper failed")

    def close(self, success):
        pending_error = None
        try:
            if self.process.poll() is None and success:
                try:
                    self.exchange("finish", "done")
                except Exception as error:
                    pending_error = error
            try:
                if self.process.stdin is not None:
                    self.process.stdin.close()
            except OSError as error:
                pending_error = pending_error or AdapterError("pointer helper disconnected")
            if self.process.poll() is None:
                self.process.send_signal(signal.SIGTERM)
            try:
                status = self.process.wait(timeout=2)
            except subprocess.TimeoutExpired:
                self.process.kill()
                status = self.process.wait()
            if success and status != 0 and pending_error is None:
                pending_error = AdapterError("pointer helper failed")
        finally:
            for signum, handler in self.previous_handlers.items():
                signal.signal(signum, handler)
        if pending_error is not None:
            raise pending_error


def execute_actions(observation_path, actions_path, env=None, uid=None, now=None, monotonic=None, which=shutil.which, helper=None):
    env = dict(os.environ if env is None else env)
    uid = os.getuid() if uid is None else uid
    observation = validate_observation(read_json(observation_path, uid))
    actions = validate_actions(read_json(actions_path, uid), observation["observation_id"])
    # Reject every statically invalid coordinate before creating a pointer or
    # sending any prefix of the batch. Live target checks still precede events.
    commands = [helper_command(action, observation) for action in actions]
    hyprctl = which("hyprctl")
    helper = helper or env.get("HYPRLAND_POINTER_HELPER") or which("hyprland-pointer-adapter-helper")
    if not hyprctl or not helper:
        raise AdapterError("required session client is unavailable")
    verify_screenshot(observation, uid, hash_contents=True)
    assert_live_target(observation, env, uid, hyprctl, now, monotonic)
    observation_deadline = observation["observed_at_monotonic_ms"] / 1000 + MAX_OBSERVATION_AGE_MS / 1000
    deadline = min(time.monotonic() + MAX_OPERATION_SECONDS, observation_deadline)
    client = HelperProcess(helper, env, deadline)
    success = False
    try:
        client.exchange(None, "ready")
        for command in commands:
            # The query/dispatch pair cannot be atomic in Hyprland. Repeating
            # this fail-closed gate before every event narrows that race while
            # preserving one virtual pointer for down/move/up drag sequences.
            if time.monotonic() >= deadline:
                raise AdapterError("operation timed out")
            assert_live_target(observation, env, uid, hyprctl, now, monotonic)
            if time.monotonic() >= deadline:
                raise AdapterError("operation timed out")
            client.exchange(command, "ok")
        success = True
    finally:
        client.close(success)
    return {"schema_version": SCHEMA_VERSION, "observation_id": observation["observation_id"], "actions_completed": len(actions)}


def parser():
    result = argparse.ArgumentParser(prog="hyprland-pointer-adapter")
    commands = result.add_subparsers(dest="command", required=True)
    observe = commands.add_parser("observe")
    observe.add_argument("--screenshot", required=True)
    observe.add_argument("--target-window", required=True)
    observe.add_argument("--output", required=True)
    act = commands.add_parser("act")
    act.add_argument("--observation", required=True)
    act.add_argument("--actions", required=True)
    return result


def main(argv=None):
    arguments = parser().parse_args(argv)
    try:
        if arguments.command == "observe":
            def cancel_observation(_signum, _frame):
                raise AdapterError("observation cancelled")

            previous_handlers = {}
            try:
                for signum in (signal.SIGTERM, signal.SIGHUP):
                    previous_handlers[signum] = signal.signal(signum, cancel_observation)
                observation = create_observation(arguments.screenshot, arguments.output, arguments.target_window)
            finally:
                for signum, handler in previous_handlers.items():
                    signal.signal(signum, handler)
            result = {"schema_version": SCHEMA_VERSION, "observation_id": observation["observation_id"]}
        else:
            result = execute_actions(arguments.observation, arguments.actions)
        print(json.dumps(result, sort_keys=True))
        return 0
    except AdapterError as error:
        print(f"hyprland-pointer-adapter: {error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
