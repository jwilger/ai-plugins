# Codex MicroVM development environment

This project supports running Codex inside a project-specific NixOS MicroVM.
The VM is the primary process and filesystem boundary: Codex runs with `--yolo`
and passwordless guest `sudo`, while the host exposes only the checkout,
project-owned persistent state, and explicitly approved extra shares.

The repository root flake is the single source for the existing development
shell, VM definition, launcher packages, host module, and checks. Routine
launches never evaluate the live checkout on the host.

## Trust boundary

The owner reviews an exact Git revision and asks Nix to evaluate only that
immutable revision. Nix builds the installer and launcher closure in the store;
the installer then writes stable wrappers under the owner's home, outside the
agent-writable repository. The wrappers execute the installed store closure and
treat the current checkout and `.codex-vm/` only as data.

Do not run `nix run .#codex-vm-installer`, `nix run .#vm-codex`, `nix develop`,
`direnv`, or another project-controlled command on the host to establish or
launch this boundary. Those forms evaluate the live checkout before the VM
exists.

| Surface                                                             | Trust classification                                                  |
| ------------------------------------------------------------------- | --------------------------------------------------------------------- |
| Exact reviewed Git revision and its `flake.lock`                    | Trusted installation input                                            |
| Nix and the installed Nix-store closure                             | Trusted host infrastructure                                           |
| `nixosModules.codex-vm-host` imported from an exact revision        | Trusted host integration                                              |
| `~/.local/bin/vm-codex` and `vm-shell` managed wrappers             | Trusted owner files                                                   |
| Live checkout, including `flake.nix`, `.envrc`, and `codex-vm.json` | Agent-writable data during routine launch                             |
| `.codex-vm/`                                                        | Agent-writable persistent data; never host-executed or host-evaluated |
| `~/.config/ai-plugins/codex-vm-shares.json`                         | Owner-controlled share authorization                                  |

## Install or update from an exact revision

Inspect and approve a full 40-character commit ID, then run:

```shell
revision=<reviewed-40-character-commit>
nix run "github:jwilger/ai-plugins/$revision#codex-vm-installer" -- \
  install --revision "$revision"
```

The installer rejects a mutable source and rejects a revision argument that
does not match the revision embedded by Nix. It installs stable wrappers in
`${XDG_BIN_HOME:-$HOME/.local/bin}` and revisioned links plus the atomic
`current` link in
`${XDG_DATA_HOME:-$HOME/.local/share}/ai-plugins/codex-vm/`. Ensure the binary
directory is on `PATH`.

To update, review a new exact revision and repeat the same command with that
revision. Existing wrappers switch atomically to the new store closure. Old
revision links remain available for inspection or manual rollback; removing
them is an owner maintenance action, not part of routine installation.

For declarative NixOS integration, pin the input to the reviewed revision in an
owner-controlled system flake:

```nix
{
  inputs.ai-plugins.url =
    "github:jwilger/ai-plugins/<reviewed-40-character-commit>";

  outputs = { nixpkgs, ai-plugins, ... }: {
    nixosConfigurations.my-host = nixpkgs.lib.nixosSystem {
      system = "x86_64-linux";
      modules = [
        ai-plugins.nixosModules.codex-vm-host
        { programs.aiPlugins.codexVm.enable = true; }
      ];
    };
  };
}
```

The host integration currently supports `x86_64-linux`.

## First start and authentication

From this checkout:

```shell
vm-codex status --json
vm-codex init
vm-codex login
vm-codex
```

`init` creates the project state without invoking Nix or executing project
files. On first initialization only, it copies the allowlisted global Codex
preferences `model`, `model_reasoning_effort`, `model_verbosity`, `personality`,
`plan_mode_reasoning_effort`, `web_search`, `file_opener`, and
`hide_agent_reasoning`. It excludes authentication, providers, MCP servers,
project settings, history, transcripts, caches, logs, and memories. Later host
preference changes are not synchronized, and guest changes never write back.

`login` starts the VM if necessary and runs Codex device authentication inside
the guest. Authentication persists in the VM-owned `/home/codex` volume; host
Codex credentials are not copied or mounted.

## Daily commands and lifecycle

```shell
vm-codex                 # start or reconnect, then run Codex --yolo in /work
vm-shell                 # start or reconnect, then open a guest login shell
vm-codex start           # start without attaching
vm-codex status --json   # inspect project identity and lifecycle state
vm-codex stop            # stop QEMU and virtiofsd for this project
```

Both entry commands resolve the same canonical checkout identity, state, SSH
key, known-host record, and loopback-only forwarded SSH port. They do not read
the owner's SSH configuration, reuse host SSH keys, forward an SSH agent, or
enable password login. `vm-shell` starts the VM when it is not already running.

Remote control is deliberately separate from SSH lifecycle management:

```shell
vm-codex remote-control start
vm-codex remote-control pair
vm-codex remote-control stop
```

These commands invoke the supported Codex remote-control flow inside the guest
over the project-specific SSH channel. Treat the paired external client as able
to control the guest Codex process. It receives no direct host filesystem,
process, device, socket, or credential access; protect pairing output and stop
remote control when it is not needed.

## Persistent state and reset

All project-specific durable VM state lives in the Git-ignored `.codex-vm/`
directory:

- `nix-store-overlay.img`: VM-owned writable Nix store overlay;
- `codex-home.img`: persistent `/home/codex`, including Codex auth and state;
- `ssh/`: project client key and pinned guest known-host record;
- `ssh-keys/`: guest host key and the client public key shared read-only;
- `bootstrap/`: one-time preferences, captured envrc data, and share manifest;
- `extra-shares/`: fixed empty export slots populated only inside the launcher
  sandbox;
- QEMU and virtiofsd logs plus lifecycle lock files.

Ephemeral PID records, image links, QMP and virtiofs sockets, and the selected
SSH port live under
`${XDG_RUNTIME_DIR:-/run/user/$UID}/ai-plugins-codex-vm/<project-id>/`. Keeping
Unix sockets outside the checkout ensures guest-side flake evaluation never
encounters unsupported filesystem objects. They are recreated on every start
and are not backup or migration state.

Back up `.codex-vm/` only as sensitive project state. To reset it, first run
`vm-codex stop`, then deliberately remove `.codex-vm/`; this deletes the guest
home, authentication, package state, keys, and VM disks and cannot be undone.
Run `vm-codex init` and `vm-codex login` afterward.

## Project environment

The launcher snapshots the applicable ancestor and project `.envrc` files as
private data without executing them on the host. The guest evaluates the
captured chain in an isolated mirror, computes only variables introduced,
changed, or unset by that chain, rejects host-boundary paths and variables, and
materializes the result into ordinary guest login and Codex environments.
Ambient host variables are not forwarded. Restart the VM to refresh changed
`.envrc` input.

Inside the guest, enter the repository's pinned development shell when needed:

```shell
nix develop
```

Package-manager caches and global installs use the persistent guest home. The
host-oriented `.dependencies/` overrides and worktree-copy behavior were
removed; legacy `.dependencies/` remains ignored only to contain old local
configuration until the owner deletes it.

## Explicit extra shares

No extra share is enabled by project configuration alone. A committed
`codex-vm.json` request must match an exact owner authorization outside the
checkout.

Project request:

```json
{
  "schemaVersion": 1,
  "shares": [
    {
      "source": "/absolute/canonical/reference",
      "mountPoint": "/mnt/reference",
      "mode": "ro"
    }
  ]
}
```

Find the project ID with `vm-codex status --json`, then authorize the exact
tuple in `~/.config/ai-plugins/codex-vm-shares.json`:

```json
{
  "schemaVersion": 1,
  "projects": {
    "project-0123456789abcdef": [
      {
        "source": "/absolute/canonical/reference",
        "mountPoint": "/mnt/reference",
        "mode": "ro"
      }
    ]
  }
}
```

Sources must already exist and resolve canonically; mount points must be direct
children of `/mnt`; modes are exactly `ro` or `rw`; and at most four of each
mode are supported. The authorization file must be an unlinked regular file
outside the checkout. Stop and restart the VM after changing shares. Removing
a live share fails safely while a guest process still uses it.

## Filesystem, privilege, and network limits

The default writable host share is this checkout at `/work`. The host home,
host `/tmp`, SSH agent, Docker or Podman sockets, process namespaces, arbitrary
devices, KVM device, and implicit credentials are absent. QEMU runs without
host device passthrough. The guest root is otherwise ephemeral, with separate
persistent volumes for `/home/codex` and the writable Nix store overlay.

Codex and the app server run with `--yolo`, and the `codex` account has
unrestricted passwordless sudo inside the guest. Guest root is therefore not a
secondary security boundary; the MicroVM and explicit shares are.

QEMU user networking supplies outbound Internet access through the standard
`10.0.2.0/24` network. The guest uses the fixed SLiRP address `10.0.2.15` and
DNS endpoint `10.0.2.3`. Guest nftables allow established traffic and QEMU DNS,
then reject private, loopback, link-local, multicast, and unique-local
destination ranges before allowing other outbound traffic. After QEMU starts,
the trusted launcher adds SSH forwarding through QMP from an automatically
selected `127.0.0.1` host port to guest port 22. The guest firewall does not
expose port 22 on a LAN interface.

Residual limits remain: QEMU, virtiofsd, Nix, the kernel, and the loopback SSH
forward are trusted host infrastructure; Internet and configured DNS endpoints
observe guest traffic; public services can relay traffic to private systems;
and an explicitly authorized read-write share permits guest root to modify that
host path. Do not treat the VM as protection from vulnerabilities in those
trusted components or from deliberate owner authorization.

## Flake and bootstrap audit

The root `flake.nix` and `flake.lock` remain because they pin the development
toolchain, MicroVM modules, Codex package, launch closure, host module, and
tests. The normal guest workflow retains `nix develop`, package locks, direnv
support, and warm `.direnv/` worktree caches for reproducibility and correct
guest behavior. None are used to establish the host boundary.

Checkout-local npm/Cargo global prefixes, checkout-local package-manager cache
overrides, `.dependencies/` bootstrap copying, and eval state under
`.dependencies/` were removed because the VM-owned home now provides that
isolation. Eval artifacts remain disposable under `.evals/`; that location is
unrelated to the VM trust boundary.
