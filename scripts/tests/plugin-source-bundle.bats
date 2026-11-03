#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  FIXTURE="$BATS_TEST_TMPDIR"
  VERSION=1.0.0
  ARCHIVE="tiber-v$VERSION-source.tar.gz"
  mkdir -p "$FIXTURE/payload/tiber-v$VERSION-source" "$FIXTURE/bin" "$FIXTURE/cache" "$FIXTURE/isolated/plugins/tiber"
  printf '%s\n' "$VERSION" >"$FIXTURE/payload/tiber-v$VERSION-source/.plugin-version"
  mkdir -p "$FIXTURE/payload/tiber-v$VERSION-source/rust/crates/tiber-git"
  cp "$ROOT/plugins/tiber/rust/crates/tiber-git/Cargo.toml" "$FIXTURE/payload/tiber-v$VERSION-source/rust/crates/tiber-git/"
  cat >"$FIXTURE/bin/curl" <<'SH'
#!/usr/bin/env bash
while (($#)); do
  case "$1" in
    --output) output=$2; shift 2 ;;
    --*) shift ;;
    *) url=$1; shift ;;
  esac
done
cp "$SOURCE_FIXTURE/${url##*/}" "$output"
SH
  chmod +x "$FIXTURE/bin/curl"
  export SOURCE_FIXTURE="$FIXTURE" PATH="$FIXTURE/bin:$PATH"
  pack
}

pack() {
  tar -C "$FIXTURE/payload" -czf "$FIXTURE/$ARCHIVE" "tiber-v$VERSION-source"
  (cd "$FIXTURE" && sha256sum "$ARCHIVE" >"$ARCHIVE.sha256")
}

prepare() {
  bash -c 'set -euo pipefail
    source "$1"
    plugin_prepare_source tiber "$2" 1.0.0 "$3" https://release.invalid
    test -f "$source_root/rust/crates/tiber-git/Cargo.toml"
    printf "%s\n" "$source_root"
  ' bash "$ROOT/plugins/tiber/lib/source-bundle.sh" "$FIXTURE/isolated/plugins/tiber" "$FIXTURE/cache"
}

@test "isolated installed plugin obtains exact verified source without sibling caches" {
  run prepare
  [ "$status" -eq 0 ]
  [[ "$output" == *'/tiber-v1.0.0-source' ]]
}

@test "source checksum failure refuses extraction" {
  printf '%064d  %s\n' 0 "$ARCHIVE" >"$FIXTURE/$ARCHIVE.sha256"
  run prepare
  [ "$status" -ne 0 ]
  [[ "$output" == *source_checksum_mismatch* ]]
}

@test "source version marker mismatch is rejected" {
  printf '%s\n' 2.0.0 >"$FIXTURE/payload/tiber-v$VERSION-source/.plugin-version"
  pack
  run prepare
  [ "$status" -ne 0 ]
}

@test "source bundle cannot extract symlinks outside staging" {
  ln -s "$FIXTURE/outside" "$FIXTURE/payload/tiber-v$VERSION-source/escape"
  pack
  run prepare
  [ "$status" -ne 0 ]
  [ ! -e "$FIXTURE/outside" ]
}

@test "development-system uses the identical source verification contract" {
  cmp "$ROOT/plugins/tiber/lib/source-bundle.sh" "$ROOT/plugins/development-system/lib/source-bundle.sh"
}
