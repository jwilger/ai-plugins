import json
import socket
import sys
import time


def fail(message: str) -> None:
    raise SystemExit(f"codex-vm-qmp-forward: {message}")


if len(sys.argv) != 3:
    fail("usage: SOCKET PORT")

socket_path = sys.argv[1]
try:
    port = int(sys.argv[2])
except ValueError:
    fail("port must be decimal")
if not 1024 <= port <= 65535:
    fail("port must be between 1024 and 65535")

connection = socket.socket(socket.AF_UNIX)
deadline = time.monotonic() + 5
while True:
    try:
        connection.connect(socket_path)
        break
    except (FileNotFoundError, ConnectionRefusedError):
        if time.monotonic() >= deadline:
            fail("QMP socket did not become ready")
        time.sleep(0.05)

stream = connection.makefile("rwb", buffering=0)


def receive() -> dict:
    while True:
        line = stream.readline()
        if not line:
            fail("QMP connection closed")
        response = json.loads(line)
        if "event" not in response:
            return response


receive()
stream.write(b'{"execute":"qmp_capabilities"}\n')
capabilities = receive()
if "error" in capabilities:
    fail(f"QMP capability negotiation failed: {capabilities['error']}")

command = f"hostfwd_add codexnet tcp:127.0.0.1:{port}-:22"
stream.write(
    (json.dumps({
        "execute": "human-monitor-command",
        "arguments": {"command-line": command},
    }) + "\n").encode()
)
result = receive()
if "error" in result:
    fail(f"QMP host forwarding failed: {result['error']}")
message = result.get("return", "")
if message:
    fail(f"QEMU rejected host forwarding: {message.strip()}")
