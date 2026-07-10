development_discipline_resolve_uname() {
  local candidate
  for candidate in /run/current-system/sw/bin/uname /usr/bin/uname /bin/uname; do
    if [ -x "$candidate" ]; then
      printf '%s\n' "$candidate"
      return 0
    fi
  done
  return 1
}

detect_development_discipline_target() {
  local uname_path="${1:-}"
  if [ -z "$uname_path" ]; then
    uname_path="$(development_discipline_resolve_uname)" || return 1
  fi
  case "$("$uname_path" -s)-$("$uname_path" -m)" in
    Linux-x86_64) echo "x86_64-unknown-linux-musl" ;;
    Linux-aarch64 | Linux-arm64) echo "aarch64-unknown-linux-musl" ;;
    Darwin-x86_64) echo "x86_64-apple-darwin" ;;
    Darwin-arm64 | Darwin-aarch64) echo "aarch64-apple-darwin" ;;
    *) return 1 ;;
  esac
}
