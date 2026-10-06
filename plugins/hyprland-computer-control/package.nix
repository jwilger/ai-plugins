{ lib, stdenv, makeWrapper, pkg-config, wayland-scanner, wayland, wlr-protocols, python3, grim }:
stdenv.mkDerivation {
  pname = "hyprland-pointer-adapter";
  version = "0.1.3";
  src = ./.;
  nativeBuildInputs = [ makeWrapper pkg-config wayland-scanner ];
  buildInputs = [ wayland ];
  dontConfigure = true;
  buildPhase = ''
    runHook preBuild
    protocol=${wlr-protocols}/share/wlr-protocols/unstable/wlr-virtual-pointer-unstable-v1.xml
    wayland-scanner client-header "$protocol" wlr-virtual-pointer-unstable-v1-client-protocol.h
    wayland-scanner private-code "$protocol" wlr-virtual-pointer-unstable-v1-protocol.c
    $CC $NIX_CFLAGS_COMPILE -I. -std=c11 -Wall -Wextra -Werror \
      -o hyprland-pointer-adapter-helper src/hyprland-pointer-adapter-helper.c \
      wlr-virtual-pointer-unstable-v1-protocol.c $(pkg-config --cflags --libs wayland-client)
    runHook postBuild
  '';
  doCheck = true;
  checkPhase = ''
    runHook preCheck
    export PYTHONDONTWRITEBYTECODE=1
    ${python3}/bin/python3 -m unittest discover -s tests -p 'test_*.py'
    $CC $NIX_CFLAGS_COMPILE -I. -std=c11 -Wall -Wextra -Werror -Wno-unused-function \
      -DPOINTER_HELPER_UNIT_TEST -o pointer-helper-unit src/hyprland-pointer-adapter-helper.c \
      wlr-virtual-pointer-unstable-v1-protocol.c $(pkg-config --cflags --libs wayland-client)
    ./pointer-helper-unit
    $CC $NIX_CFLAGS_COMPILE -I. -std=c11 -Wall -Wextra -Werror -Wno-unused-function \
      -DPOINTER_HELPER_UNIT_TEST -o roundtrip-lifecycle tests/roundtrip_lifecycle.c \
      wlr-virtual-pointer-unstable-v1-protocol.c $(pkg-config --cflags --libs wayland-client)
    ./roundtrip-lifecycle
    runHook postCheck
  '';
  installPhase = ''
    runHook preInstall
    install -Dm500 hyprland-pointer-adapter-helper "$out/libexec/hyprland-pointer-adapter-helper"
    install -Dm400 scripts/hyprland-pointer-adapter.py "$out/libexec/hyprland-pointer-adapter.py"
    makeWrapper ${python3}/bin/python3 "$out/bin/hyprland-pointer-adapter" \
      --add-flags "$out/libexec/hyprland-pointer-adapter.py" \
      --set HYPRLAND_POINTER_HELPER "$out/libexec/hyprland-pointer-adapter-helper" \
      --prefix PATH : ${lib.makeBinPath [ grim ]}
    runHook postInstall
  '';
  doInstallCheck = true;
  installCheckPhase = ''
    "$out/bin/hyprland-pointer-adapter" --help >/dev/null
    test -x "$out/libexec/hyprland-pointer-adapter-helper"
    test ! -e "$out/bin/hyprland-pointer-adapter-helper"
  '';
  meta = {
    description = "Guarded session-local pointer control for Hyprland on Wayland";
    license = lib.licenses.agpl3Plus;
    platforms = lib.platforms.linux;
    mainProgram = "hyprland-pointer-adapter";
  };
}
