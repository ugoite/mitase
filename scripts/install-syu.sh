#!/usr/bin/env bash
# FEAT-INSTALL-001

set -euo pipefail

DEFAULT_REPOSITORY="ugoite/syu"
DEFAULT_PACKAGE_HOST="ghcr.io"
DEFAULT_PACKAGE_SCHEME="https"
DEFAULT_VERSION_SELECTOR="alpha"
tmp_dir=""

cleanup_tmp_dir() {
  if [[ -n "${tmp_dir:-}" ]]; then
    rm -rf "$tmp_dir"
  fi
}

require_command() {
  local command_name="$1"
  if ! command -v "$command_name" >/dev/null 2>&1; then
    echo "required command not found: $command_name" >&2
    exit 1
  fi
}

find_python() {
  if command -v python3 >/dev/null 2>&1; then
    command -v python3
    return 0
  fi

  if command -v python >/dev/null 2>&1; then
    command -v python
    return 0
  fi

  echo "python3 or python is required by the installer" >&2
  exit 1
}

resolve_repository() {
  if [[ -n "${SYU_REPOSITORY:-}" ]]; then
    printf '%s\n' "$SYU_REPOSITORY"
    return 0
  fi

  if [[ -n "${GITHUB_REPOSITORY:-}" ]]; then
    printf '%s\n' "$GITHUB_REPOSITORY"
    return 0
  fi

  printf '%s\n' "$DEFAULT_REPOSITORY"
}

resolve_package_host() {
  printf '%s\n' "${SYU_PACKAGE_HOST:-$DEFAULT_PACKAGE_HOST}"
}

resolve_package_scheme() {
  printf '%s\n' "${SYU_PACKAGE_SCHEME:-$DEFAULT_PACKAGE_SCHEME}"
}

resolve_package_repository() {
  local repository="$1"

  if [[ -n "${SYU_PACKAGE_REPOSITORY:-}" ]]; then
    printf '%s\n' "$SYU_PACKAGE_REPOSITORY"
    return 0
  fi

  printf '%s\n' "$repository"
}

normalize_version_selector() {
  local version="${SYU_VERSION:-$DEFAULT_VERSION_SELECTOR}"

  case "$version" in
    latest | alpha | beta | stable)
      printf '%s\n' "$version"
      ;;
    v*)
      printf '%s\n' "$version"
      ;;
    *)
      printf 'v%s\n' "$version"
      ;;
  esac
}

resolve_target_triple() {
  local os_name arch_name

  os_name="$(uname -s)"
  arch_name="$(uname -m)"

  case "$arch_name" in
    x86_64 | amd64) arch_name="x86_64" ;;
    arm64 | aarch64) arch_name="aarch64" ;;
    *)
      echo "unsupported architecture: $arch_name" >&2
      exit 1
      ;;
  esac

  case "$os_name" in
    Darwin) printf '%s\n' "${arch_name}-apple-darwin" ;;
    Linux) printf '%s\n' "${arch_name}-unknown-linux-gnu" ;;
    MINGW* | MSYS* | CYGWIN*) printf '%s\n' "${arch_name}-pc-windows-msvc" ;;
    *)
      echo "unsupported operating system: $os_name" >&2
      exit 1
      ;;
  esac
}

resolve_install_dir() {
  local os_name

  if [[ -n "${SYU_INSTALL_DIR:-}" ]]; then
    printf '%s\n' "$SYU_INSTALL_DIR"
    return 0
  fi

  os_name="$(uname -s)"

  case "$os_name" in
    MINGW* | MSYS* | CYGWIN*)
      if [[ -n "${LOCALAPPDATA:-}" ]]; then
        printf '%s\n' "${LOCALAPPDATA}/Programs/syu/bin"
      else
        printf '%s\n' "${HOME}/AppData/Local/Programs/syu/bin"
      fi
      ;;
    *)
      printf '%s\n' "${HOME}/.local/bin"
      ;;
  esac
}

resolve_archive_name() {
  local target="$1"

  if [[ "$target" == *windows* ]]; then
    printf 'syu-%s.zip\n' "$target"
  else
    printf 'syu-%s.tar.gz\n' "$target"
  fi
}

resolve_binary_name() {
  local target="$1"

  if [[ "$target" == *windows* ]]; then
    printf 'syu.exe\n'
  else
    printf 'syu\n'
  fi
}

fetch_registry_token() {
  local package_host="$1"
  local package_repository="$2"
  local package_scheme="$3"
  local python_bin response service

  python_bin="$(find_python)"
  service="${package_host%%:*}"
  response="$(
    curl -fsSL \
      "${package_scheme}://${package_host}/token?scope=repository:${package_repository}:pull&service=${service}"
  )" || return 1

  printf '%s' "$response" | "$python_bin" -c '
import json
import sys

payload = json.load(sys.stdin)
token = payload.get("token") or payload.get("access_token")
if not token:
    raise SystemExit("failed to resolve package registry token")
print(token)
'
}

resolve_package_tag() {
  local package_host="$1"
  local package_scheme="$2"
  local package_repository="$3"
  local target="$4"
  local python_bin response selector token

  python_bin="$(find_python)"
  selector="$(normalize_version_selector)"
  token="$(fetch_registry_token "$package_host" "$package_repository" "$package_scheme")"
  response="$(
    curl -fsSL \
      -H "Authorization: Bearer $token" \
      "${package_scheme}://${package_host}/v2/${package_repository}/tags/list"
  )" || return 1

  printf '%s' "$response" | "$python_bin" -c '
import json
import re
import sys

selector, target = sys.argv[1], sys.argv[2]
payload = json.load(sys.stdin)
tags = payload.get("tags") or []
pattern = re.compile(r"^(v\d+\.\d+\.\d+(?:-(alpha|beta)\.\d+)?)__(.+)$")
version_pattern = re.compile(r"^v(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta)\.(\d+))?$")

candidates = []
for tag in tags:
    match = pattern.fullmatch(tag)
    if not match:
        continue

    version = match.group(1)
    tag_target = match.group(3)
    if tag_target != target:
        continue

    version_match = version_pattern.fullmatch(version)
    if not version_match:
        continue

    major, minor, patch = (int(version_match.group(index)) for index in (1, 2, 3))
    prerelease_type = version_match.group(4)
    prerelease_number = int(version_match.group(5) or 0)
    prerelease_rank = {"alpha": 0, "beta": 1, None: 2}[prerelease_type]
    candidates.append(
        (
            major,
            minor,
            patch,
            prerelease_rank,
            prerelease_number,
            prerelease_type,
            version,
            tag,
        )
    )

if selector.startswith("v"):
    expected = f"{selector}__{target}"
    if expected not in tags:
        raise SystemExit(f"package tag not found: {expected}")
    print(expected)
    raise SystemExit(0)

if selector == "latest":
    filtered = candidates
elif selector == "stable":
    filtered = [candidate for candidate in candidates if candidate[5] is None]
elif selector == "alpha":
    filtered = [candidate for candidate in candidates if candidate[5] == "alpha"]
elif selector == "beta":
    filtered = [candidate for candidate in candidates if candidate[5] == "beta"]
else:
    raise SystemExit(f"unsupported version selector: {selector}")

if not filtered:
    raise SystemExit(
        f"no package tag matched selector {selector!r} for target {target}"
    )

filtered.sort(key=lambda candidate: candidate[:5], reverse=True)
print(filtered[0][7])
' "$selector" "$target"
}

download_package_archive() {
  local package_host="$1"
  local package_scheme="$2"
  local package_repository="$3"
  local package_tag="$4"
  local archive_name="$5"
  local archive_path="$6"
  local digest manifest python_bin token

  python_bin="$(find_python)"
  token="$(fetch_registry_token "$package_host" "$package_repository" "$package_scheme")"
  manifest="$(
    curl -fsSL \
      -H "Authorization: Bearer $token" \
      -H "Accept: application/vnd.oci.image.manifest.v1+json, application/vnd.oci.artifact.manifest.v1+json, application/vnd.oras.artifact.manifest.v1+json" \
      "${package_scheme}://${package_host}/v2/${package_repository}/manifests/${package_tag}"
  )" || return 1
  digest="$(
    printf '%s' "$manifest" | "$python_bin" -c '
import json
import sys

archive_name = sys.argv[1]
manifest = json.load(sys.stdin)
layers = manifest.get("layers") or []

for layer in layers:
    annotations = layer.get("annotations") or {}
    if annotations.get("org.opencontainers.image.title") == archive_name:
        print(layer["digest"])
        raise SystemExit(0)

for layer in layers:
    annotations = layer.get("annotations") or {}
    title = annotations.get("org.opencontainers.image.title", "")
    if title.endswith(".tar.gz") or title.endswith(".zip"):
        print(layer["digest"])
        raise SystemExit(0)

if len(layers) == 1:
    print(layers[0]["digest"])
    raise SystemExit(0)

raise SystemExit(f"unable to find archive layer for {archive_name}")
' "$archive_name"
  )"

  curl -fsSL \
    -H "Authorization: Bearer $token" \
    "${package_scheme}://${package_host}/v2/${package_repository}/blobs/${digest}" \
    -o "$archive_path"
}

fetch_release_catalog() {
  local repository="$1"

  curl -fsSL \
    -H "Accept: application/vnd.github+json" \
    "https://api.github.com/repos/${repository}/releases?per_page=100"
}

resolve_release_asset_url() {
  local repository="$1"
  local archive_name="$2"
  local python_bin release_catalog selector

  python_bin="$(find_python)"
  selector="$(normalize_version_selector)"
  release_catalog="$(fetch_release_catalog "$repository")"

  printf '%s' "$release_catalog" | "$python_bin" -c '
import json
import re
import sys

selector, archive_name = sys.argv[1], sys.argv[2]
releases = json.load(sys.stdin)
version_pattern = re.compile(r"^v(\d+)\.(\d+)\.(\d+)(?:-(alpha|beta)\.(\d+))?$")

candidates = []
for release in releases:
    if release.get("draft"):
        continue
    tag = release.get("tag_name")
    if not tag:
        continue

    match = version_pattern.fullmatch(tag)
    if not match:
        continue

    major, minor, patch = (int(match.group(index)) for index in (1, 2, 3))
    prerelease_type = match.group(4)
    prerelease_number = int(match.group(5) or 0)
    prerelease_rank = {"alpha": 0, "beta": 1, None: 2}[prerelease_type]
    candidates.append(
        (
            major,
            minor,
            patch,
            prerelease_rank,
            prerelease_number,
            prerelease_type,
            tag,
            release,
        )
    )

if selector.startswith("v"):
    filtered = [candidate for candidate in candidates if candidate[6] == selector]
elif selector == "latest":
    filtered = candidates
elif selector == "stable":
    filtered = [candidate for candidate in candidates if candidate[5] is None]
elif selector == "alpha":
    filtered = [candidate for candidate in candidates if candidate[5] == "alpha"]
elif selector == "beta":
    filtered = [candidate for candidate in candidates if candidate[5] == "beta"]
else:
    raise SystemExit(f"unsupported version selector: {selector}")

if not filtered:
    raise SystemExit(f"no release matched selector {selector!r}")

filtered.sort(key=lambda candidate: candidate[:5], reverse=True)
release = filtered[0][7]

for asset in release.get("assets") or []:
    if asset.get("name") == archive_name:
        print(asset["browser_download_url"])
        raise SystemExit(0)

raise SystemExit(f"release asset not found: {archive_name}")
' "$selector" "$archive_name"
}

download_release_archive() {
  local repository="$1"
  local archive_name="$2"
  local archive_path="$3"
  local asset_url

  asset_url="$(resolve_release_asset_url "$repository" "$archive_name")"

  curl -fsSL "$asset_url" -o "$archive_path"
}

download_distribution_archive() {
  local repository="$1"
  local package_host="$2"
  local package_scheme="$3"
  local package_repository="$4"
  local target="$5"
  local archive_name="$6"
  local archive_path="$7"
  local package_error_log package_tag

  package_error_log="${tmp_dir}/package-download.log"

  if package_tag="$(
    resolve_package_tag "$package_host" "$package_scheme" "$package_repository" "$target" 2>"$package_error_log"
  )"; then
    if download_package_archive \
      "$package_host" \
      "$package_scheme" \
      "$package_repository" \
      "$package_tag" \
      "$archive_name" \
      "$archive_path" \
      2>>"$package_error_log"; then
      return 0
    fi
  fi

  if [[ -s "$package_error_log" ]]; then
    cat "$package_error_log" >&2
  fi
  echo "package download unavailable, falling back to GitHub release assets" >&2
  download_release_archive "$repository" "$archive_name" "$archive_path"
}

extract_archive() {
  local archive_path="$1"
  local destination_dir="$2"
  local python_bin

  mkdir -p "$destination_dir"

  case "$archive_path" in
    *.tar.gz)
      require_command tar
      tar -C "$destination_dir" -xzf "$archive_path"
      ;;
    *.zip)
      python_bin="$(find_python)"
      "$python_bin" - "$archive_path" "$destination_dir" <<'PY'
import sys
import zipfile
from pathlib import Path

archive_path = Path(sys.argv[1])
destination_dir = Path(sys.argv[2])

with zipfile.ZipFile(archive_path) as archive:
    archive.extractall(destination_dir)
PY
      ;;
    *)
      echo "unsupported archive format: $archive_path" >&2
      exit 1
      ;;
  esac
}

install_binary() {
  local source_path="$1"
  local destination_path="$2"
  local python_bin

  mkdir -p "$(dirname "$destination_path")"

  if command -v install >/dev/null 2>&1; then
    install -m 0755 "$source_path" "$destination_path"
    return 0
  fi

  python_bin="$(find_python)"
  "$python_bin" - "$source_path" "$destination_path" <<'PY'
import os
import shutil
import stat
import sys
from pathlib import Path

source_path = Path(sys.argv[1])
destination_path = Path(sys.argv[2])
destination_path.parent.mkdir(parents=True, exist_ok=True)
shutil.copy2(source_path, destination_path)

mode = destination_path.stat().st_mode
destination_path.chmod(mode | stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR)
PY
}

install_syu() {
  local repository
  local package_host
  local package_repository
  local package_scheme
  local target
  local install_dir
  local binary_name
  local archive_name
  local archive_path
  local extracted_dir

  require_command curl

  repository="$(resolve_repository)"
  package_host="$(resolve_package_host)"
  package_scheme="$(resolve_package_scheme)"
  package_repository="$(resolve_package_repository "$repository")"
  target="$(resolve_target_triple)"
  install_dir="$(resolve_install_dir)"
  binary_name="$(resolve_binary_name "$target")"
  archive_name="$(resolve_archive_name "$target")"

  tmp_dir="$(mktemp -d)"
  extracted_dir="${tmp_dir}/extract"
  archive_path="${tmp_dir}/${archive_name}"

  trap cleanup_tmp_dir EXIT

  download_distribution_archive \
    "$repository" \
    "$package_host" \
    "$package_scheme" \
    "$package_repository" \
    "$target" \
    "$archive_name" \
    "$archive_path"
  extract_archive "$archive_path" "$extracted_dir"
  install_binary "${extracted_dir}/${binary_name}" "${install_dir}/${binary_name}"

  echo "installed ${binary_name} to ${install_dir}/${binary_name}"
}

install_syu "$@"
