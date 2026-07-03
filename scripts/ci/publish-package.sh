#!/usr/bin/env bash
# FEAT-RELEASE-001

set -euo pipefail

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "required command not found: $command_name" >&2
    exit 1
  fi
}

publish_package_artifact() {
  local registry="$1"
  local package_repository="$2"
  local package_tag="$3"
  local archive_path="$4"
  local checksum_path="${archive_path}.sha256"
  local source_url="https://github.com/${GITHUB_REPOSITORY:-ugoite/syu}"

  require_command oras

  if [[ ! -f "$archive_path" ]]; then
    echo "missing package archive: $archive_path" >&2
    exit 1
  fi

  if [[ ! -f "$checksum_path" ]]; then
    echo "missing package checksum: $checksum_path" >&2
    exit 1
  fi

  oras push \
    --artifact-type application/vnd.syu.release.v1 \
    --annotation "org.opencontainers.image.source=${source_url}" \
    --annotation "org.opencontainers.image.version=${package_tag%%__*}" \
    "${registry}/${package_repository}:${package_tag}" \
    "${archive_path}:application/vnd.syu.archive.layer.v1" \
    "${checksum_path}:text/plain"
}

publish_package_artifact "$@"
