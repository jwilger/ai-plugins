detect_tiber_target() {
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) echo "x86_64-unknown-linux-gnu" ;;
    Linux-aarch64 | Linux-arm64) echo "aarch64-unknown-linux-gnu" ;;
    Darwin-x86_64) echo "x86_64-apple-darwin" ;;
    Darwin-arm64 | Darwin-aarch64) echo "aarch64-apple-darwin" ;;
    *) return 1 ;;
  esac
}
