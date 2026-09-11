#!/usr/bin/env bash

set -euo pipefail

if [[ "$#" -ne 4 ]]; then
  echo "usage: $0 <version> <target> <archive> <fixture>" >&2
  exit 2
fi

version="$1"
target="$2"
archive="$3"
fixture="$4"

case "$target" in
  x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu | \
    x86_64-apple-darwin | aarch64-apple-darwin)
    ;;
  *)
    echo "unsupported release target: $target" >&2
    exit 1
    ;;
esac

expected_archive="mitase-${version}-${target}.tar.gz"

if [[ "$(basename "$archive")" != "$expected_archive" ]]; then
  echo "unexpected release archive name: $(basename "$archive")" >&2
  exit 1
fi

if [[ ! -f "$archive" ]]; then
  echo "release archive does not exist: $archive" >&2
  exit 1
fi

checksum_file="${archive}.sha256"
if [[ ! -f "$checksum_file" ]]; then
  echo "release archive checksum is missing: $checksum_file" >&2
  exit 1
fi
if command -v sha256sum >/dev/null 2>&1; then
  (cd "$(dirname "$archive")" && sha256sum -c "$(basename "$checksum_file")")
elif command -v shasum >/dev/null 2>&1; then
  (cd "$(dirname "$archive")" && shasum -a 256 -c "$(basename "$checksum_file")")
else
  echo "release archive smoke test requires sha256sum or shasum" >&2
  exit 1
fi

entry_count=0
entry_name=""
while IFS= read -r entry; do
  entry_count=$((entry_count + 1))
  entry_name="$entry"
done < <(tar -tzf "$archive")
if [[ "$entry_count" -ne 1 || "$entry_name" != "mitase" ]]; then
  echo "release archive must contain exactly the mitase executable" >&2
  printf 'entries: %s\n' "${entry_name:-<empty>}" >&2
  exit 1
fi

temp_dir="$(mktemp -d "${TMPDIR:-/tmp}/mitase-package-smoke.XXXXXX")"
trap 'rm -rf "$temp_dir"' EXIT

tar -xzf "$archive" -C "$temp_dir"
binary="$temp_dir/mitase"
if [[ ! -x "$binary" ]]; then
  echo "packaged Mitase binary is not executable" >&2
  exit 1
fi

expected_version="mitase ${version#v}"
actual_version="$($binary --version)"
if [[ "$actual_version" != "$expected_version" ]]; then
  echo "packaged Mitase version mismatch" >&2
  printf 'expected: %s\nactual:   %s\n' "$expected_version" "$actual_version" >&2
  exit 1
fi

"$binary" check "$fixture" >/dev/null
