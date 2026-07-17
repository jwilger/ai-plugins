#!/usr/bin/env bats

setup() {
  [ "$(uname -s)" = Linux ] || skip "aggregate systemd scopes are Linux-only"
}

@test "Nix devshell selects its declared aggregate-scope tools" {
  [ -x "${AI_PLUGINS_SYSTEMD_RUN_BIN:-}" ]
  [ -x "${AI_PLUGINS_SYSTEMCTL_BIN:-}" ]

  systemd_run="$(realpath "$(command -v systemd-run)")"
  systemctl="$(realpath "$(command -v systemctl)")"

  [ "$systemd_run" = "$(realpath "$AI_PLUGINS_SYSTEMD_RUN_BIN")" ]
  [ "$systemctl" = "$(realpath "$AI_PLUGINS_SYSTEMCTL_BIN")" ]
  [[ "$systemd_run" =~ ^/nix/store/[0-9a-z]{32}-systemd-[^/]+/bin/systemd-run$ ]]
  [[ "$systemctl" =~ ^/nix/store/[0-9a-z]{32}-systemd-[^/]+/bin/systemctl$ ]]
}
