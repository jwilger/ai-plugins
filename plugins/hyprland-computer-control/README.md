# Hyprland computer control

A Codex skill and short-lived guarded pointer adapter for **Hyprland on
Wayland**, on Linux x86_64. This is a distinct plugin alongside
development-system; neither plugin depends on the other. No MCP server, daemon,
network listener, startup hook, input permission change, or credential bridge is
included. Loading the plugin does not install the adapter or connect an
assistant to a desktop.

The marketplace's default installer selects only development-system. After this
plugin is published to the configured `ai-plugins` marketplace, a separately
authorized CLI installation would load its skill with:

```sh
codex plugin add hyprland-computer-control@ai-plugins
```

Use the intended CLI identity for that future operation. Do not install it into
Desktop's isolated home implicitly. Plugin loading and adapter setup are
separate; the following build instructions do not install the skill.

## Prerequisites and setup

- An authorized local executor in the owner's existing unlocked Hyprland on
  Wayland session, with `XDG_SESSION_TYPE=wayland`, `XDG_RUNTIME_DIR`,
  `WAYLAND_DISPLAY`, and `HYPRLAND_INSTANCE_SIGNATURE` inherited. The runtime
  directory must be absolute, private, and user-owned. The named Wayland socket
  and `$XDG_RUNTIME_DIR/hypr/$HYPRLAND_INSTANCE_SIGNATURE/.socket.sock` must be
  same-user sockets. Each observation binds the canonical runtime directory and
  both socket device/inode identities; capture and every pointer event reject a
  changed identity. No alternate session discovery is performed.
- The running compositor's `hyprctl` on PATH, supporting JSON `locked`,
  `activewindow`, and `monitors`. The package deliberately does not substitute a
  different compositor client. Unknown lock state fails closed.
- Compositor support for grim screencopy and
  `zwlr_virtual_pointer_manager_v1`. Portal screen sharing alone does not prove
  pointer support. The source implementation was developed against Hyprland
  0.55.4; other versions need separate compatibility verification.
- The guarded `hyprland-pointer-adapter` package built from this plugin's recipe
  or verified to include equivalent safety repairs, and a harness that can display
  its screenshot files. `grim` and Python are included in the Nix runtime
  closure; Wayland client libraries and generated protocol code are compiled
  into the helper's package. No root access or `/dev/uinput` is needed.

A program name or matching `0.1.1` label is not enough to establish runtime
provenance. Building this package does not replace an older adapter already on
PATH. Check the selected executable's derivation/source identity before input;
if it is an older unpatched build or its provenance is unknown, stop for
separately authorized setup rather than silently using it.

For a local source checkout of this marketplace, build without installation:

```sh
nix build --no-link .#hyprland-computer-control
```

The root flake lock pins the build dependencies. For another Nix project,
`pkgs.callPackage /path/to/plugin/package.nix { }` builds the same source recipe
using that project's nixpkgs. Review dependency/version changes there separately.
The package exposes only `bin/hyprland-pointer-adapter`; the internal helper is
under `libexec` and must never be used to bypass the Python guard.

For persistent availability through Home Manager, the required dependency is
the **adapter package in `home.packages`** (plus the host's compatible
`hyprctl`). An example for a future explicitly authorized configuration change:

```nix
home.packages = [
  (pkgs.callPackage /path/to/plugin/package.nix { })
];
```

Plugin installation and Home Manager activation are separate actions requiring
their own authorization. No switch or installation is performed by this plugin.
The existing home's optional `jwilger.computerControlProbe.enable` installs a
read-only readiness probe, not this adapter, and is not required.

Noctalia supplies the desktop shell, notifications, lock UI, and screenshot
dependencies in the source Home Manager configuration (`grim`, `slurp`, and
`wl-clipboard`). Hyprland owns compositor state and Wayland virtual-pointer
input. This package includes grim itself, so Noctalia is optional. The home's
Voxtype module supplies `wtype` for transcription and uses Noctalia
notifications; neither Voxtype nor wtype is a pointer dependency.

For a setup explanation, distinguish these requirements from the source T14's
optional shell/transcription configuration:

| Setup item                      | Required mapping                                                                           |
| ------------------------------- | ------------------------------------------------------------------------------------------ |
| Matching guarded adapter        | `home.packages` when using Home Manager, or another explicitly authorized package setup    |
| Compatible `hyprctl`            | The running host compositor's client on PATH                                               |
| grim, Python, Wayland libraries | Supplied by this Nix recipe; no separate Noctalia installation needed                      |
| Noctalia, Voxtype, wtype        | Optional shell/transcription tools; no keyboard capability is added to this adapter        |
| Account/configuration homes     | Preserve each host's configured `CODEX_HOME` and authentication; do not copy or merge them |

When installation or switching is denied, describe the missing dependency and
proposed configuration without applying it. Having shell/transcription tools
installed does not prove the adapter, authorization, or assistant transport.

Keep each host's configured `CODEX_HOME` and authentication isolated. The source
T14 setup uses `$HOME/.codex` for CLI and `$HOME/.codex-chatgpt` for Desktop;
those are examples, not required runtime paths. Do not copy credentials, change
either host's settings, or assume cloud/dot/voice access from local plugin
availability. This plugin does not implement native Linux Computer Use.

## Observe, inspect, act

Coordinate desktop use with the owner and identify the authorized window.
The adapter never changes focus. Ask the owner to focus the target if needed.
Read `hyprctl -j activewindow` only in the authorized session to obtain its
address; check that it describes the intended application.

After capture and input have been authorized, use a private task directory and
new output paths for each observation:

```sh
umask 077
task_dir="$(mktemp -d)"
hyprland-pointer-adapter observe --target-window 0xabc \
  --screenshot "$task_dir/target.png" --output "$task_dir/observation.json"
```

`0xabc` is an example; replace it with the actual authorized focused address.
Inspect the PNG using the harness's image tool before choosing coordinates.
The PNG is a crop of the window's screen rectangle, not a private window buffer.
Overlays, rounded corners, and animation can expose unrelated content. Stop if
capture reveals unintended content or the intended control is obscured.

Create `actions.json` with mode 0600 inside the same private directory. Copy the
actual `observation_id` from the inspected observation. A click example:

```json
{
  "schema_version": 1,
  "observation_id": "REPLACE_WITH_OBSERVATION_ID",
  "actions": [
    { "type": "button", "button": "left", "state": "down", "x": 40, "y": 60 },
    { "type": "button", "button": "left", "state": "up", "x": 40, "y": 60 }
  ]
}
```

```sh
hyprland-pointer-adapter act --observation "$task_dir/observation.json" \
  --actions "$task_dir/actions.json"
```

Coordinates are integer **pixels relative to the inspected crop**, not global
logical desktop coordinates. The wrapper maps scale 1 or 2 and monitor origins,
including negative origins. Capture explicitly uses the target monitor's scale,
so a scale-1 target beside a scale-2 output remains a scale-1 crop. Other actions:

| Action   | Fields                      | Meaning                                       |
| -------- | --------------------------- | --------------------------------------------- |
| `move`   | `x`, `y`                    | Move within the crop                          |
| `button` | `x`, `y`, `button`, `state` | Left/right/middle; down/up must balance       |
| `scroll` | `x`, `y`, `dx`, `dy`        | Wheel steps, each axis -20..20; not both zero |

A drag is down/move/up in **one** action sequence and one virtual-pointer
lifetime. Use minimal scroll steps and inspect the result; positive `dy` is
sent as positive vertical wheel steps. There are at most 64 events, a 12-second
operation deadline, and a 30-second observation lifetime starting before
capture. Both wall and monotonic clocks are checked. Exit 0 returns JSON with
the observation ID or completed event count; refusal is exit 1 with a diagnostic
on stderr; invalid CLI syntax is exit 2. Partial completion is possible on error.

Re-observe after a UI change, and reconcile an uncertain outcome before retrying
any action. Never replay an old batch blindly. Remove task artifacts after use
when no longer needed; no screenshot retention service is included.

## Stop conditions and limits

The wrapper checks session identity, lock state, focus, window geometry, monitor
layout, screenshot identity/integrity, freshness, and coordinate bounds. Input
files must be bounded private regular files under a private real parent; symlinks
are rejected. Unsupported schemas, unbalanced buttons, and unknown actions fail.
Before each event it rechecks the live target and deadline. Cancellation and
failure terminate/reap the helper; the helper attempts reverse-order releases of
held buttons before destroying its short-lived virtual pointer.

The entire batch's coordinates are mapped and checked before helper startup.
A known invalid later coordinate refuses the batch without sending earlier
events. Runtime failures or live target changes can still interrupt an otherwise
valid batch after partial delivery; reconcile the outcome before retrying.
Failed post-capture session/state checks and cancellation clean up the created
screenshot when filesystem cleanup is possible. A hard kill or filesystem
failure can prevent cleanup; remove leftover private task artifacts after use.
Observation temporarily handles SIGTERM/SIGHUP as cancellation so capture
subprocesses are reaped and cleanup runs; SIGINT also unwinds through cleanup.

Stop on owner activity, refusal, changed target/session/layout/focus, ambiguous
capture, cancellation, or uncertain delivery. Do not edit observations to extend
freshness, change guard constants, call the helper directly, or substitute
unguarded input. A refusal is diagnostic evidence, not permission to bypass.

Only untransformed monitors at scale 1 or 2 are supported. The target rectangle
must fit on one monitor. Fractional scaling, rotation, and spanning windows are
rejected. The IPC query/event pair is not atomic: a focus race remains between
the final query and input delivery. Dynamic UI changes within the same window
are not detected by geometry checks. Screenshots can contain overlays.
Compositor disconnects can prevent release delivery; no atomic rollback or
exactly-once guarantee exists. The adapter cannot determine task authorization
from application content; that boundary belongs to the skill and owner.

There is **no keyboard/text action** in this API. Earlier guarded `wtype`
experiments were separate and are not bundled. Do not invent keyboard JSON or
fall back to raw wtype. The plugin adds no generic Linux, X11, portal/libei,
native Computer Use, cloud, dot, or voice backend.

## Provenance and verification

Runtime sources derive from the owner's home repository implementation
associated with merged pointer PR16,
`44e214c0bbb20934f4dde7e2180a6e3c9c65fa0b`. They are bundled here rather than
loaded from personal absolute paths. Original offline tests remain unchanged.
The bundled Python guard opens input files nonblocking so the existing
regular-file check rejects a FIFO without waiting for a writer; a CLI regression
test covers this refusal. It also prevalidates every coordinate before helper
startup, preventing malformed batches from emitting prefix events. Terminal
review also added explicit target-scale capture and cleanup across rejected or
cancelled observations, with mixed-scale, post-capture-failure, and SIGINT,
SIGTERM/SIGHUP cancellation
CLI regressions. It identified a native callback-lifetime
defect: pending sync callbacks could retain stack-backed listener data after a
timeout or cancellation. The bundled C helper now destroys those callbacks on
every unsuccessful roundtrip exit; offline lifecycle tests cover timeout,
listener/flush/dispatch failure, cancellation, and subsequent cleanup.
The build recipe retains its compile flags,
guarded wrapper, private helper placement, and dependency closure. Additional
CLI tests use fake session sockets and fake clients only.

The owner reports bounded click, drag, and corrected scroll acceptance passed.
The home document's corrected-scroll acceptance-pending statement is stale.
This is prior user-reported device evidence, **not fresh live validation of this
plugin build**. No desktop capture, input, focus change, or activation is part of
the offline tests. New runtime behavior or a changed compositor requires a
separately authorized bounded scratch-window acceptance test before claiming
live compatibility.

Read-only comparison before the bundled callback-lifetime repair found the
source T14's installed guard and C source identical to the PR16 baseline. The
installed runtime does not include the bundled FIFO-open, batch-preflight,
target-scale/capture-cleanup, or callback-lifetime repairs. Native dependency
closures: the installed helper links Wayland 1.25.0, while this marketplace's
pinned build links Wayland 1.26.0 (and a different glibc store closure). Source
identity alone does not establish binary identity or transfer earlier live acceptance
to this newly built closure. That closure still needs separately authorized
bounded scratch-window acceptance before claiming live validation.

Run `nix build --no-link .#hyprland-computer-control` or
`just hyprland-computer-control` for Python safety/CLI tests, C protocol-event
tests, warnings-as-errors compilation, and installed-layout checks.
Behavior fixtures live under
`evals/fixtures/behavior/hyprland-computer-control/`. Wiring validation is
separate from provider-backed behavior evidence; never use `just evals` for a
local-only request because it shares results externally.

Packaging follows [official OpenAI documentation](https://developers.openai.com/plugins/build/plugins)
and the repository's vendored Agent Plugins 1.0.0 schema: root `plugin.json`,
root `skills/`, and optional client presentation under `extensions.com.openai`.
No `mcp.json` is needed for this CLI-based workflow.
