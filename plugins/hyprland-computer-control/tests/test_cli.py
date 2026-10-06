"""Exercise observe/act through the CLI using only temporary fake clients."""

import hashlib
import json
import os
from pathlib import Path
import socket
import subprocess
import sys
import tempfile
import unittest


ADAPTER = Path(__file__).resolve().parents[1] / "scripts/hyprland-pointer-adapter.py"


class CliTests(unittest.TestCase):
    def setUp(self):
        self.temporary = tempfile.TemporaryDirectory()
        self.addCleanup(self.temporary.cleanup)
        self.root = Path(self.temporary.name)
        runtime = self.root / "runtime"
        runtime.mkdir(mode=0o700)
        (runtime / "hypr" / "fixture").mkdir(parents=True)
        for path in (runtime / "wayland-fixture", runtime / "hypr/fixture/.socket.sock"):
            client = socket.socket(socket.AF_UNIX)
            client.bind(str(path))
            self.addCleanup(client.close)
        self.state = {
            "locked": {"locked": False},
            "activewindow": {"address": "0xabc", "monitor": 1, "at": [10, 10], "size": [100, 80]},
            "monitors": [{"id": 1, "name": "fixture", "x": 0, "y": 0,
                          "width": 800, "height": 600, "scale": 1, "transform": 0}],
        }
        self.save_state()
        self.env = os.environ | {
            "XDG_SESSION_TYPE": "wayland", "XDG_RUNTIME_DIR": str(runtime),
            "WAYLAND_DISPLAY": "wayland-fixture", "HYPRLAND_INSTANCE_SIGNATURE": "fixture",
            "FIXTURE_ROOT": str(self.root), "PATH": str(self.root),
            "HYPRLAND_POINTER_HELPER": str(self.root / "helper"),
        }
        self.executable("hyprctl", """
import json, os, sys
from pathlib import Path
state = json.loads((Path(os.environ['FIXTURE_ROOT']) / 'state.json').read_text())
assert sys.argv[1] == '-j'
print(json.dumps(state[sys.argv[2]]))
""")
        self.executable("grim", """
import json, os, signal, sys
from pathlib import Path
args = sys.argv[1:]
assert args[args.index('-g') + 1] == '10,10 100x80'
state_path = Path(os.environ['FIXTURE_ROOT']) / 'state.json'
state = json.loads(state_path.read_text())
scale = int(args[args.index('-s') + 1]) if '-s' in args else max(m['scale'] for m in state['monitors'])
Path(args[-1]).write_bytes(b'\\x89PNG\\r\\n\\x1a\\n' + b'\\0\\0\\0\\rIHDR' + (100 * scale).to_bytes(4, 'big') + (80 * scale).to_bytes(4, 'big'))
if os.environ.get('FIXTURE_FAIL_POST_CAPTURE') == '1':
    state['locked'] = {}
    state_path.write_text(json.dumps(state))
cancel_signal = {'1': signal.SIGINT, 'TERM': signal.SIGTERM, 'HUP': signal.SIGHUP}.get(os.environ.get('FIXTURE_CANCEL_CAPTURE'))
if cancel_signal:
    os.kill(os.getppid(), cancel_signal)
""")
        self.executable("helper", """
import os, signal, sys
from pathlib import Path
signal.signal(signal.SIGTERM, lambda *_: None)
(Path(os.environ['FIXTURE_ROOT']) / 'helper-started').write_text('started')
print('ready', flush=True)
for line in sys.stdin:
    if line == 'finish\\n':
        print('done', flush=True)
        os._exit(0)
    with (Path(os.environ['FIXTURE_ROOT']) / 'events').open('a') as stream:
        stream.write(line)
    print('ok', flush=True)
""")

    def executable(self, name, body):
        path = self.root / name
        path.write_text(f"#!{sys.executable}\n" + body)
        path.chmod(0o700)

    def save_state(self):
        (self.root / "state.json").write_text(json.dumps(self.state))

    def run_cli(self, *arguments, env=None):
        return subprocess.run([sys.executable, str(ADAPTER), *arguments],
                              env=self.env if env is None else env,
                              capture_output=True, text=True, timeout=10, check=False)

    def observe(self, target="0xabc"):
        return self.run_cli("observe", "--target-window", target,
                            "--screenshot", str(self.root / "image.png"),
                            "--output", str(self.root / "observation.json"))

    def prepare(self, actions=None):
        result = self.observe()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.observation = json.loads((self.root / "observation.json").read_text())
        value = {"schema_version": 1, "observation_id": self.observation["observation_id"],
                 "actions": actions or [{"type": "move", "x": 20, "y": 30}]}
        self.write_private("actions.json", value)

    def write_private(self, name, value):
        path = self.root / name
        path.write_text(json.dumps(value))
        path.chmod(0o600)

    def act(self):
        return self.run_cli("act", "--observation", str(self.root / "observation.json"),
                            "--actions", str(self.root / "actions.json"))

    def assert_refused(self, result, message):
        self.assertEqual(result.returncode, 1, result.stdout + result.stderr)
        self.assertIn(message, result.stderr)
        self.assertEqual(result.stdout, "")
        self.assertFalse((self.root / "events").exists(), "refusal emitted input")

    def test_observe_then_drag_and_scroll_returns_receipt(self):
        self.prepare([
            {"type": "button", "button": "left", "state": "down", "x": 20, "y": 30},
            {"type": "move", "x": 40, "y": 50},
            {"type": "button", "button": "left", "state": "up", "x": 40, "y": 50},
            {"type": "scroll", "x": 40, "y": 50, "dx": 0, "dy": 2},
        ])
        screenshot = self.root / "image.png"
        self.assertEqual(screenshot.stat().st_mode & 0o777, 0o600)
        self.assertEqual((self.root / "observation.json").stat().st_mode & 0o777, 0o600)
        self.assertEqual(self.observation["screenshot"]["sha256"], hashlib.sha256(screenshot.read_bytes()).hexdigest())
        result = self.act()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(json.loads(result.stdout), {
            "schema_version": 1, "observation_id": self.observation["observation_id"], "actions_completed": 4,
        })
        self.assertEqual((self.root / "events").read_text().splitlines(), [
            "button 60 80 1600 1200 272 1", "move 100 120 1600 1200",
            "button 100 120 1600 1200 272 0", "scroll 100 120 1600 1200 0 2",
        ])

    def test_wrong_target_refuses_capture(self):
        self.assert_refused(self.observe("0xdef"), "not focused")
        self.assertFalse((self.root / "image.png").exists())

    def test_non_wayland_session_refuses_capture(self):
        self.assert_refused(self.run_cli("observe", "--target-window", "0xabc",
                            "--screenshot", str(self.root / "image.png"),
                            "--output", str(self.root / "observation.json"),
                            env=self.env | {"XDG_SESSION_TYPE": "x11"}), "Wayland")

    def test_changed_focus_refuses_input(self):
        self.prepare()
        self.state["activewindow"]["address"] = "0xdef"
        self.save_state()
        self.assert_refused(self.act(), "changed")

    def test_same_named_sockets_in_another_runtime_refuse_input_before_helper(self):
        self.prepare()
        runtime = self.root / "other-runtime"
        runtime.mkdir(mode=0o700)
        (runtime / "hypr" / "fixture").mkdir(parents=True)
        for path in (runtime / "wayland-fixture", runtime / "hypr/fixture/.socket.sock"):
            endpoint = socket.socket(socket.AF_UNIX)
            endpoint.bind(str(path))
            self.addCleanup(endpoint.close)
        result = self.run_cli("act", "--observation", str(self.root / "observation.json"),
                              "--actions", str(self.root / "actions.json"),
                              env=self.env | {"XDG_RUNTIME_DIR": str(runtime)})
        self.assert_refused(result, "graphical session changed")
        self.assertFalse((self.root / "helper-started").exists())

    def test_locked_session_refuses_input(self):
        self.prepare()
        self.state["locked"]["locked"] = True
        self.save_state()
        self.assert_refused(self.act(), "locked")

    def test_stale_observation_refuses_input(self):
        self.prepare()
        self.observation["observed_at_monotonic_ms"] -= 31000
        self.write_private("observation.json", self.observation)
        self.assert_refused(self.act(), "stale")

    def test_tampered_screenshot_refuses_input(self):
        self.prepare()
        with (self.root / "image.png").open("ab") as stream:
            stream.write(b"tamper")
        self.assert_refused(self.act(), "screenshot")

    def test_unbalanced_buttons_refuse_input(self):
        self.prepare([{"type": "button", "button": "left", "state": "down", "x": 20, "y": 30}])
        self.assert_refused(self.act(), "held")

    def test_keyboard_is_not_a_pointer_action(self):
        self.prepare([{"type": "key", "key": "Return"}])
        self.assert_refused(self.act(), "unsupported action")

    def test_public_action_file_refuses_input(self):
        self.prepare()
        (self.root / "actions.json").chmod(0o644)
        self.assert_refused(self.act(), "unsafe")

    def test_coordinates_outside_crop_refuse_input(self):
        self.prepare([{"type": "move", "x": 100, "y": 0}])
        self.assert_refused(self.act(), "outside")

    def test_fifo_observation_refuses_without_waiting_for_a_writer(self):
        os.mkfifo(self.root / "observation.json", mode=0o600)
        self.assert_refused(self.act(), "unsafe")

    def test_invalid_later_coordinate_refuses_entire_batch_before_helper_startup(self):
        self.prepare([
            {"type": "button", "button": "left", "state": "down", "x": 20, "y": 30},
            {"type": "move", "x": 100, "y": 30},
            {"type": "button", "button": "left", "state": "up", "x": 20, "y": 30},
        ])
        self.assert_refused(self.act(), "outside")
        self.assertFalse((self.root / "helper-started").exists())

    def test_mixed_scale_capture_uses_target_monitor_scale(self):
        self.state["monitors"].append({"id": 2, "name": "peer", "x": 800, "y": 0,
                                      "width": 1600, "height": 1200, "scale": 2, "transform": 0})
        self.save_state()
        result = self.observe()
        self.assertEqual(result.returncode, 0, result.stderr)
        value = json.loads((self.root / "observation.json").read_text())
        self.assertEqual((value["screenshot"]["width"], value["screenshot"]["height"]), (100, 80))

    def test_post_capture_state_failure_removes_rejected_image(self):
        self.env["FIXTURE_FAIL_POST_CAPTURE"] = "1"
        self.assert_refused(self.observe(), "lock state")
        self.assertFalse((self.root / "image.png").exists())
        self.assertFalse((self.root / "observation.json").exists())

    def test_capture_cancellation_removes_reserved_image(self):
        self.env["FIXTURE_CANCEL_CAPTURE"] = "1"
        result = self.observe()
        self.assertNotEqual(result.returncode, 0)
        self.assertFalse((self.root / "image.png").exists())
        self.assertFalse((self.root / "observation.json").exists())

    def test_capture_sigterm_removes_reserved_image(self):
        self.env["FIXTURE_CANCEL_CAPTURE"] = "TERM"
        self.assert_refused(self.observe(), "cancelled")
        self.assertFalse((self.root / "image.png").exists())
        self.assertFalse((self.root / "observation.json").exists())

    def test_capture_sighup_removes_reserved_image(self):
        self.env["FIXTURE_CANCEL_CAPTURE"] = "HUP"
        self.assert_refused(self.observe(), "cancelled")
        self.assertFalse((self.root / "image.png").exists())
        self.assertFalse((self.root / "observation.json").exists())


if __name__ == "__main__":
    unittest.main()
