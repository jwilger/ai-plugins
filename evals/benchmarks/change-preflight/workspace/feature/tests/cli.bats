@test "export command produces task data" {
  run tool export
  [ "$status" -eq 0 ]
}
