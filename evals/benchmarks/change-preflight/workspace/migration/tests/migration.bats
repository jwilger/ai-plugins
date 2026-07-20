@test "upgrade and rollback preserve task data" {
  run tool migrate --verify-rollback
  [ "$status" -eq 0 ]
}
