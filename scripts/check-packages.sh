#!/usr/bin/env bash
set -euo pipefail

test -f LICENSE
test -f README.md

for package in packages/*; do
  test -d "$package"
  test -f "$package/README.md"
  test -d "$package/scripts"
  find "$package/scripts" -type f -name '*.klc' -print -quit | grep -q .
done

if [[ -n "${KALCITE_CLI_MANIFEST:-}" ]]; then
  cargo run --quiet --manifest-path "$KALCITE_CLI_MANIFEST" -p kalcite-cli -- \
    check packages/tween/scripts/tween.klc
fi
