#!/usr/bin/env bash
# FEAT-RELEASE-001

set -euo pipefail

find_python() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
    return 0
  fi

  if command -v python >/dev/null 2>&1; then
    command -v python
    return 0
  fi

  echo "python3 or python is required to package releases" >&2
  exit 1
}

write_sha256() {
  local archive_path="$1"
  local checksum_path="$2"

  if command -v sha256sum >/dev/null 2>&1; then
    (cd "$(dirname "$archive_path")" && sha256sum "$(basename "$archive_path")") >"$checksum_path"
  else
    (cd "$(dirname "$archive_path")" && shasum -a 256 "$(basename "$archive_path")") >"$checksum_path"
  fi
}

package_release_artifact() {
  local target="$1"
  local binary_path="$2"
  local output_dir="$3"
  local version="$4"
  local python_bin
  local asset_base
  local archive_path

  if [[ ! -f "$binary_path" ]]; then
    echo "missing release binary: $binary_path" >&2
    exit 1
  fi

  if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([\.-][A-Za-z0-9.-]+)?$ ]]; then
    echo "release version must be a version tag: $version" >&2
    exit 1
  fi

  mkdir -p "$output_dir"
  asset_base="mitase-${version}-${target}"

  if [[ "$target" == *windows* ]]; then
    archive_path="${output_dir}/${asset_base}.zip"
    python_bin="$(find_python)"
    "$python_bin" - "$binary_path" "$archive_path" <<'PY'
import sys
import zipfile
from pathlib import Path

binary_path = Path(sys.argv[1])
archive_path = Path(sys.argv[2])

with zipfile.ZipFile(archive_path, "w", compression=zipfile.ZIP_DEFLATED) as archive:
    archive.write(binary_path, arcname=binary_path.name)
PY
  else
    archive_path="${output_dir}/${asset_base}.tar.gz"
    tar -C "$(dirname "$binary_path")" -czf "$archive_path" "$(basename "$binary_path")"
  fi

  write_sha256 "$archive_path" "${archive_path}.sha256"
}

if [[ "$#" -ne 4 ]]; then
  echo "usage: $0 <target> <binary> <output-dir> <version>" >&2
  exit 2
fi

package_release_artifact "$@"
