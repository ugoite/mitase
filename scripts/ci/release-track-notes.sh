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

release_track() {
  local tag="$1"

  case "$tag" in
    *-alpha.*) printf 'alpha\n' ;;
    *-beta.*) printf 'beta\n' ;;
    *) printf 'stable\n' ;;
  esac
}

previous_track_tag() {
  local repository="$1"
  local current_tag="$2"
  local track="$3"
  gh api "repos/${repository}/releases?per_page=100" | python3 -c '
import json
import sys

current_tag, desired_track = sys.argv[1:3]
releases = json.load(sys.stdin)

def track_for(tag: str) -> str:
    if "-alpha." in tag:
        return "alpha"
    if "-beta." in tag:
        return "beta"
    return "stable"

for release in releases:
    tag = release.get("tag_name") or ""
    if not tag or tag == current_tag:
        continue
    if release.get("draft"):
        continue
    if track_for(tag) != desired_track:
        continue
    print(tag)
    break
' "$current_tag" "$track"
}

generate_release_notes() {
  local repository="$1"
  local tag="$2"
  local previous_tag="$3"
  local payload

  if [[ -n "$previous_tag" ]]; then
    payload="$(gh api \
      -X POST \
      "repos/${repository}/releases/generate-notes" \
      -f tag_name="$tag" \
      -f target_commitish="main" \
      -f previous_tag_name="$previous_tag")"
  else
    payload="$(gh api \
      -X POST \
      "repos/${repository}/releases/generate-notes" \
      -f tag_name="$tag" \
      -f target_commitish="main")"
  fi

  printf '%s' "$payload" | python3 -c 'import json, sys; print(json.load(sys.stdin)["body"])'
}

main() {
  require_command gh
  require_command python3

  local tag="${1:-${RELEASE_TAG:-}}"
  local repository="${GITHUB_REPOSITORY:-ugoite/syu}"

  if [[ -z "$tag" ]]; then
    echo "release tag is required" >&2
    exit 1
  fi

  local track previous_tag notes_file
  track="$(release_track "$tag")"
  previous_tag="$(previous_track_tag "$repository" "$tag" "$track")"
  notes_file="$(mktemp)"

  generate_release_notes "$repository" "$tag" "$previous_tag" >"$notes_file"
  gh release edit "$tag" --notes-file "$notes_file"

  rm -f "$notes_file"
  echo "updated ${tag} release notes using the ${track} track"
}

main "$@"
