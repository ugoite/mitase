#!/usr/bin/env bash
# FEAT-RELEASE-001

set -euo pipefail

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
  local asset_base
  local archive_path

  if [[ ! -f "$binary_path" ]]; then
    echo "missing release binary: $binary_path" >&2
    exit 1
  fi

  case "$target" in
    x86_64-unknown-linux-gnu | aarch64-unknown-linux-gnu | \
      x86_64-apple-darwin | aarch64-apple-darwin)
      ;;
    *)
      echo "unsupported release target: $target" >&2
      exit 1
      ;;
  esac

  if [[ ! "$version" =~ ^v[0-9]+\.[0-9]+\.[0-9]+([\.-][A-Za-z0-9.-]+)?$ ]]; then
    echo "release version must be a version tag: $version" >&2
    exit 1
  fi

  mkdir -p "$output_dir"
  asset_base="mitase-${version}-${target}"

  archive_path="${output_dir}/${asset_base}.tar.gz"
  tar -C "$(dirname "$binary_path")" -czf "$archive_path" "$(basename "$binary_path")"

  write_sha256 "$archive_path" "${archive_path}.sha256"
}

if [[ "$#" -ne 4 ]]; then
  echo "usage: $0 <target> <binary> <output-dir> <version>" >&2
  exit 2
fi

package_release_artifact "$@"
