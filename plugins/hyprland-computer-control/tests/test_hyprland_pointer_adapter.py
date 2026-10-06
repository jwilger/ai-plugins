"""Offline tests for the guarded pointer adapter; no desktop clients run."""

import importlib.util
import json
import os
from pathlib import Path
import stat
import tempfile
import time
from types import SimpleNamespace
import unittest
from unittest.mock import patch


SOURCE = Path(__file__).resolve().parents[1] / "scripts" / "hyprland-pointer-adapter.py"
SPEC = importlib.util.spec_from_file_location("pointer_adapter", SOURCE)
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


def monitor(identifier=1, name="eDP-1", x=0, y=0, width=2880, height=1800, scale=2, transform=0):
    return {
        "id": identifier, "name": name, "x": x, "y": y, "width": width,
        "height": height, "scale": scale, "transform": transform,
    }


def window(address="0xabc", monitor_id=1, x=100, y=50, width=800, height=600):
    return {"address": address, "monitor": monitor_id, "x": x, "y": y, "width": width, "height": height}


def observation(monitors=None, focused=None, screenshot=None):
    monitors = monitors or [monitor()]
    focused = focused or window()
    screenshot = screenshot or {
        "path": "/private/scratch.png", "width": focused["width"] * 2,
        "height": focused["height"] * 2, "sha256": "a" * 64,
        "device": 1, "inode": 2, "size": 24, "mtime_ns": 3,
    }
    return {
        "schema_version": 1, "observation_id": "1" * 32,
        "observed_at_unix_ms": 1_000_000, "observed_at_monotonic_ms": 500_000,
        "session": {"uid": 1000, "wayland_display": "wayland-1", "hyprland_instance_signature": "instance_1"},
        "focused_window": focused, "monitors": monitors,
        "target_monitor": next(item["name"] for item in monitors if item["id"] == focused["monitor"]),
        "screenshot": screenshot,
    }


class SchemaAndMappingTests(unittest.TestCase):
    def test_scale_two_window_crop_maps_through_negative_unequal_layout(self):
        monitors = [
            monitor(1, "left", -1920, 200, 1920, 1080, 1),
            monitor(2, "target", 0, -100, 3000, 2000, 2),
        ]
        focused = window(monitor_id=2, x=200, y=50, width=800, height=600)
        value = observation(monitors, focused, {
            "path": "/private/scratch.png", "width": 1600, "height": 1200,
            "sha256": "a" * 64, "device": 1, "inode": 2, "size": 24, "mtime_ns": 3,
        })
        # Global logical bounds: (-1920,-100)..(1500,1280). Crop pixel
        # (600,400) maps to logical (500,250), then to normalized half-units.
        self.assertEqual(MODULE.move_command({"type": "move", "x": 600, "y": 400}, value),
                         "move 4840 700 6840 2760")
        fields = [int(part) for part in MODULE.move_command(
            {"type": "move", "x": 1599, "y": 1199}, value).split()[1:]]
        self.assertTrue(all(0 <= field <= 0xFFFFFFFF for field in fields))

    def test_crop_bounds_are_strict(self):
        value = observation()
        self.assertIn("move ", MODULE.move_command({"type": "move", "x": 1599, "y": 1199}, value))
        for x, y in ((1600, 0), (0, 1200)):
            with self.subTest(x=x, y=y), self.assertRaises(MODULE.AdapterError):
                MODULE.move_command({"type": "move", "x": x, "y": y}, value)

    def test_transforms_fractional_scale_and_straddling_window_fail(self):
        with self.assertRaises(MODULE.AdapterError):
            MODULE.canonical_monitors([monitor(transform=1)])
        with self.assertRaises(MODULE.AdapterError):
            MODULE.canonical_monitors([monitor(scale=1.5)])
        state = {"monitors": MODULE.canonical_monitors([monitor()]),
                 "focused_window": window(x=1400, width=100)}
        with self.assertRaises(MODULE.AdapterError):
            MODULE.target_monitor(state)

    def test_actions_are_typed_bounded_and_balanced(self):
        valid = {"schema_version": 1, "observation_id": "1" * 32, "actions": [
            {"type": "button", "button": "left", "state": "down", "x": 4, "y": 5},
            {"type": "move", "x": 4, "y": 5},
            {"type": "button", "button": "left", "state": "up", "x": 4, "y": 5},
        ]}
        self.assertEqual(len(MODULE.validate_actions(valid, "1" * 32)), 3)
        for actions in (
            [{"type": "button", "button": "left", "state": "down", "x": 1, "y": 1}],
            [{"type": "button", "button": "left", "state": "up", "x": 1, "y": 1}],
            [{"type": "exec", "command": "hyprctl dispatch"}],
            [{"type": "scroll", "dx": 21, "dy": 0, "x": 1, "y": 1}],
            [{"type": "move", "x": 1.5, "y": 0}],
        ):
            with self.subTest(actions=actions), self.assertRaises(MODULE.AdapterError):
                MODULE.validate_actions({"schema_version": 1, "observation_id": "1" * 32, "actions": actions}, "1" * 32)

    def test_stale_and_future_observations_fail(self):
        value = observation()
        self.assertTrue(MODULE.observation_is_fresh(value, now=lambda: 1029.0, monotonic=lambda: 529.0))
        self.assertFalse(MODULE.observation_is_fresh(value, now=lambda: 1031.0, monotonic=lambda: 531.0))
        self.assertFalse(MODULE.observation_is_fresh(value, now=lambda: 999.0, monotonic=lambda: 499.0))


class StateGateTests(unittest.TestCase):
    def setUp(self):
        self.value = observation()
        self.env = {"safe": "fixture"}

    def assert_rejected(self, session=None, state=None, fresh=True):
        with patch.object(MODULE, "session_snapshot", return_value=session or self.value["session"]), \
             patch.object(MODULE, "observation_is_fresh", return_value=fresh), \
             patch.object(MODULE, "verify_screenshot"), \
             patch.object(MODULE, "live_state", side_effect=state if isinstance(state, Exception) else None,
                          return_value=state if isinstance(state, dict) else {
                              "focused_window": self.value["focused_window"], "monitors": self.value["monitors"]}):
            with self.assertRaises(MODULE.AdapterError):
                MODULE.assert_live_target(self.value, self.env, 1000, "hyprctl")

    def test_changed_session_focus_geometry_layout_and_lock_are_rejected(self):
        self.assert_rejected(session=self.value["session"] | {"wayland_display": "wayland-2"})
        changed_focus = {"focused_window": self.value["focused_window"] | {"address": "0xdef"}, "monitors": self.value["monitors"]}
        self.assert_rejected(state=changed_focus)
        changed_geometry = {"focused_window": self.value["focused_window"] | {"x": 101}, "monitors": self.value["monitors"]}
        self.assert_rejected(state=changed_geometry)
        changed_layout = {"focused_window": self.value["focused_window"], "monitors": [self.value["monitors"][0] | {"x": 1}]}
        self.assert_rejected(state=changed_layout)
        self.assert_rejected(state=MODULE.AdapterError("session is locked"))
        self.assert_rejected(fresh=False)

    def test_slow_state_query_crossing_ttl_is_rejected_after_query(self):
        clock = {"wall": 1000.0, "monotonic": 500.0}
        self.value["observed_at_unix_ms"] = 1_000_000
        self.value["observed_at_monotonic_ms"] = 500_000

        def delayed_state(_hyprctl, _env):
            clock["wall"] += 31
            clock["monotonic"] += 31
            return {"focused_window": self.value["focused_window"], "monitors": self.value["monitors"]}

        with patch.object(MODULE, "session_snapshot", return_value=self.value["session"]), \
             patch.object(MODULE, "verify_screenshot"), \
             patch.object(MODULE, "live_state", side_effect=delayed_state):
            with self.assertRaisesRegex(MODULE.AdapterError, "stale"):
                MODULE.assert_live_target(
                    self.value, self.env, 1000, "hyprctl",
                    now=lambda: clock["wall"], monotonic=lambda: clock["monotonic"],
                )


class FileAndCaptureTests(unittest.TestCase):
    def test_private_regular_file_required_and_symlink_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            source = root / "input.json"
            source.write_text("{}")
            source.chmod(0o600)
            _, _, data = MODULE.read_owned_file(str(source), os.getuid(), 100)
            self.assertEqual(data, b"{}")
            alias = root / "alias.json"
            alias.symlink_to(source)
            with self.assertRaises(MODULE.AdapterError):
                MODULE.read_owned_file(str(alias), os.getuid(), 100)
            source.chmod(0o640)
            with self.assertRaises(MODULE.AdapterError):
                MODULE.read_owned_file(str(source), os.getuid(), 100)
            private = root / "private"
            private.mkdir(mode=0o700)
            private_file = private / "value"
            private_file.write_text("ok")
            private_file.chmod(0o600)
            linked_parent = root / "linked"
            linked_parent.symlink_to(private, target_is_directory=True)
            with self.assertRaises(MODULE.AdapterError):
                MODULE.read_owned_file(str(linked_parent / "value"), os.getuid(), 100)

    def test_capture_failure_removes_reserved_output(self):
        with tempfile.TemporaryDirectory() as temporary:
            screenshot = Path(temporary) / "capture.png"
            completed = __import__("subprocess").CompletedProcess([], 1)
            with patch.object(MODULE.subprocess, "run", return_value=completed):
                with self.assertRaises(MODULE.AdapterError):
                    MODULE.capture_screenshot("grim", str(screenshot), window(), {}, os.getuid())
            self.assertFalse(screenshot.exists())

    def test_observation_clock_starts_before_capture_and_postchecks(self):
        clock = {"wall": 1000.0, "monotonic": 500.0}
        state = {
            "focused_window": window(),
            "monitors": MODULE.canonical_monitors([monitor()]),
        }
        png = b"\x89PNG\r\n\x1a\n" + b"\0\0\0\rIHDR" + (1600).to_bytes(4, "big") + (1200).to_bytes(4, "big")
        info = SimpleNamespace(st_dev=1, st_ino=2, st_size=len(png), st_mtime_ns=3)

        def capture(*_args):
            clock["wall"] += 10
            clock["monotonic"] += 10
            return Path("/private/capture.png")

        with patch.object(MODULE, "session_snapshot", return_value={
                 "uid": 1000, "wayland_display": "wayland-1", "hyprland_instance_signature": "instance_1"}), \
             patch.object(MODULE, "live_state", return_value=state), \
             patch.object(MODULE, "capture_screenshot", side_effect=capture), \
             patch.object(MODULE, "read_owned_file", return_value=(Path("/private/capture.png"), info, png)), \
             patch.object(MODULE, "atomic_private_json"):
            value = MODULE.create_observation(
                "/private/capture.png", "/private/observation.json", "0xabc",
                env={}, uid=1000, now=lambda: clock["wall"],
                monotonic=lambda: clock["monotonic"], which=lambda name: name,
            )
        self.assertEqual(value["observed_at_unix_ms"], 1_000_000)
        self.assertEqual(value["observed_at_monotonic_ms"], 500_000)


class HelperLifecycleTests(unittest.TestCase):
    class Pipe:
        def __init__(self):
            self.writes = []

        def write(self, value): self.writes.append(value)
        def flush(self): pass
        def close(self): pass

    class Process:
        def __init__(self, status=None):
            self.status = status
            self.signals = []
            self.killed = False
            self.stdin = None
            self.stdout = None

        def poll(self): return self.status
        def send_signal(self, signum): self.signals.append(signum); self.status = 1
        def wait(self, timeout=None): return self.status
        def kill(self): self.killed = True; self.status = -9

    def test_error_and_cancellation_signal_helper(self):
        client = object.__new__(MODULE.HelperProcess)
        client.process = self.Process()
        client.previous_handlers = {}
        client.deadline = time.monotonic() + 1
        client.close(False)
        self.assertEqual(client.process.signals, [MODULE.signal.SIGTERM])

        client.process = self.Process()
        client._forward_signal(MODULE.signal.SIGINT, None)
        self.assertEqual(client.process.signals, [MODULE.signal.SIGINT])

    def test_finish_failure_still_terminates_and_reaps(self):
        client = object.__new__(MODULE.HelperProcess)
        client.process = self.Process()
        client.previous_handlers = {}
        client.deadline = time.monotonic() + 1
        client.cancelled = False
        client.exchange = lambda command, expected: (_ for _ in ()).throw(MODULE.AdapterError("failed"))
        with self.assertRaises(MODULE.AdapterError):
            client.close(True)
        self.assertEqual(client.process.signals, [MODULE.signal.SIGTERM])
        self.assertIsNotNone(client.process.status)

    def test_expired_deadline_never_writes_helper_command(self):
        client = object.__new__(MODULE.HelperProcess)
        client.process = self.Process()
        client.process.stdin = self.Pipe()
        client.process.stdout = object()
        client.previous_handlers = {}
        client.deadline = time.monotonic() - 1
        client.cancelled = False
        with self.assertRaisesRegex(MODULE.AdapterError, "timed out"):
            client.exchange("button 1 1 2 2 272 1", "ok")
        self.assertEqual(client.process.stdin.writes, [])


if __name__ == "__main__":
    unittest.main()
