#!/usr/bin/env bash

set -euo pipefail

node --test tiber/scripts/check-lint-policy.test.mjs
node tiber/scripts/check-lint-policy.mjs tiber
