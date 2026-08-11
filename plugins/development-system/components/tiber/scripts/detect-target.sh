detect_tiber_target() {
  case "$(uname -s)-$(uname -m)" in
    Linux-x86_64) echo "x86_64-unknown-linux-gnu" ;;
    *) return 1 ;;
  esac
}
