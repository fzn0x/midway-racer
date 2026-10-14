#!/usr/bin/env bash
# Downloads the classic CP/M 8080 CPU diagnostics used by
# crates/i8080/tests/cpm.rs and verifies their checksums.
#
# Usage: scripts/fetch-cpu-tests.sh [destination]
set -euo pipefail

readonly COMMIT="274ffd700b81baabea99b0963bc1260b67132185"
readonly BASE_URL="https://raw.githubusercontent.com/superzazu/8080/${COMMIT}/cpu_tests"
readonly DEST="${1:-$(dirname "$0")/../crates/i8080/tests/roms}"

# SHA-256 checksum and file name of every program.
readonly PROGRAMS="
9561c6fb6c99efe3de00eb77e4044fd102151058b39ac2d7bce10483838a08e7 TST8080.COM
18eb3c79cba42c0718f160be6a1853cb64cdce7aa47d65780189a57bdd98c4e0 8080PRE.COM
e61a9a75348c774486c2207080ea4effbf6c2367fdace31b0731081a4144030b CPUTEST.COM
6e3286e11bb1a8f47b8ee1280b4a067be813193363e3223c99b0d21912f44aeb 8080EXM.COM
"

sha256() {
  if command -v sha256sum >/dev/null; then
    sha256sum "$1" | cut -d ' ' -f 1
  else
    shasum -a 256 "$1" | cut -d ' ' -f 1
  fi
}

mkdir -p "$DEST"
while read -r checksum name; do
  [ -n "$name" ] || continue
  curl --fail --silent --show-error --location --retry 3 \
    --output "$DEST/$name" "$BASE_URL/$name"
  actual="$(sha256 "$DEST/$name")"
  if [ "$actual" != "$checksum" ]; then
    echo "error: checksum mismatch for $name" >&2
    rm -f "$DEST/$name"
    exit 1
  fi
  echo "fetched $name"
done <<< "$PROGRAMS"
