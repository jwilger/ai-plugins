# ai-plugins

A plugin marketplace for Codex. Use Development System to plan, implement, review,
and deliver repository changes; use Hyprland Computer Control for guarded local
screenshots and pointer tasks. Codex and Linux x86_64 are the required targets.

## Plugin catalog

| Plugin                                                                   | Harness | Description                                                                                  | Version |
| ------------------------------------------------------------------------ | ------- | -------------------------------------------------------------------------------------------- | ------- |
| [development-system](plugins/development-system/README.md)               | Codex   | Advisory repository setup, structured multi-agent review, and delivery workflow.             | 10.1.0  |
| [tiber](plugins/tiber/README.md)                                         | Codex   | Independent Git-backed task board, CLI, MCP server, and dashboard.                           | 1.2.1   |
| [hyprland-computer-control](plugins/hyprland-computer-control/README.md) | Codex   | Guarded screenshots and pointer actions for an authorized local Hyprland on Wayland session. | 0.1.4   |

## Using the marketplace (Codex)

Codex-facing marketplace metadata lives in
[`.agents/plugins/marketplace.json`](.agents/plugins/marketplace.json), and each
plugin has a portable root `plugin.json` manifest and owns its root `mcp.json`.
Development System registers Development Discipline; Tiber registers its own
task server. Consuming projects do not need MCP registrations. The installer
installs both plugins by default. Install or update from `main` with:

```shell
curl -fsSL https://raw.githubusercontent.com/jwilger/ai-plugins/main/install.sh | sh
```

The installer offers optional GitHub and CodeRabbit plugins and bootstraps
shared Tiber SSH-agent forwarding by environment-variable name. Restart Codex
from a shell with the current agent after installation or agent rotation.
Plugin-only Tiber startup can bootstrap the same stable dispatcher automatically;
after one activation restart, future Tiber upgrades need no installer rerun.
Existing 1.1.x registrations need one migration through the current installer
or installed helper.
Use `sh install.sh --dry-run` to preview the commands.

After installation, start a new thread and run the setup skill from the target
repository's primary checkout.
Development System's installed `SessionStart` hook checks and repairs its own
Development Discipline binary. Tiber's launcher independently checks and repairs
its task binary. Each plugin uses its own versioned binary cache. Linux x86_64
downloads checksum-verified releases; hosts without a prebuilt release build the
plugin's locked Cargo workspace from dependency-complete source. The setup
skill checks Development Discipline and, when Tiber integration is enabled,
requires the separately installed Tiber companion. Setup also generates
and installs a stack-specific project Lefthook configuration with both
pre-commit and pre-push jobs; installed workflow guidance relies on those Git
hooks rather than duplicating their verification commands manually.

## Using the plugins well

Follow the [Development System user guide](plugins/development-system/README.md)
for greenfield and brownfield walkthroughs, example requests, and daily habits.
The [Hyprland guide](plugins/hyprland-computer-control/README.md) covers separate
adapter setup, authorized desktop tasks, and supported limits.

The installer needs Codex, Python 3.11 or later, and a shell. Binary repair needs Bash,
curl, tar, checksum tools, and flock on Linux; source fallback needs Cargo.
Nix is required for repository development, not ordinary Codex plugin use.
Run the installer again to update its configured GitHub/main marketplace.
Tiber can also be installed independently as `tiber@ai-plugins`.

## Contributing and support

See [CONTRIBUTING.md](CONTRIBUTING.md) and the [evaluation guide](docs/evaluations.md).
The canonical home is [GitHub](https://github.com/jwilger/ai-plugins).
Report reproducible problems through [GitHub Issues](https://github.com/jwilger/ai-plugins/issues);
support is best effort. Remove secrets and private project data from reports.

## License

Licensed under [AGPL-3.0-or-later](LICENSE).

You may use these plugins as development tools for open-source or closed-source
software, including commercial products. That use alone does not require you
to publish your project's source code.

Copying or incorporating marketplace code into a product is different. AGPL
obligations apply to covered code and derivative works when distributed;
modified versions offered for remote network interaction must offer their
corresponding source to those users. Output containing copied covered material
may retain licensing obligations. This explanation adds no license exception.
See the [GNU licensing FAQ](https://www.gnu.org/licenses/gpl-faq.en.html) and
[AGPL text](https://www.gnu.org/licenses/agpl-3.0-body.html).
