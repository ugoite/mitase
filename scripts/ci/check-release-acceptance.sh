#!/usr/bin/env bash
# FEAT-RELEASE-002

set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

version="$(python3 - <<'PY'
import re
from pathlib import Path

contents = Path("Cargo.toml").read_text(encoding="utf-8")
section = re.search(r"(?ms)^\[workspace\.package\]\s+(.*?)(?=^\[|\Z)", contents)
if section is None:
    raise SystemExit("workspace package section is missing")
match = re.search(r'^version\s*=\s*"([^\"]+)"\s*$', section.group(1), re.MULTILINE)
if match is None:
    raise SystemExit("workspace package version is missing")
print(match.group(1))
PY
)"

run_mitase() {
  cargo run --locked --quiet -- "$@"
}

run_focused_test() {
  cargo test --locked --quiet --test v1_cli "$1"
}

run_boundary_test() {
  cargo test --locked --quiet --test v1_cli public_cli_does_not_expose
}

assert_v1_rejected() {
  local command_name="$1"
  shift
  local output_path="$1"
  shift
  local error_path="$1"
  shift

  if run_mitase "$@" >"$output_path" 2>"$error_path"; then
    echo "${command_name} unexpectedly accepted a v1 source" >&2
    return 1
  fi

  OUTPUT_PATH="$output_path" COMMAND_NAME="$command_name" python3 - <<'PY'
import json
import os
from pathlib import Path

payload = json.loads(Path(os.environ["OUTPUT_PATH"]).read_text(encoding="utf-8"))
diagnostics = payload.get("diagnostics", [])
matching = [item for item in diagnostics if item.get("code") == "MITASE-SOURCE-001"]
if len(matching) != 1:
    raise SystemExit(
        f"{os.environ['COMMAND_NAME']} did not report exactly one MITASE-SOURCE-001"
    )
diagnostic = matching[0]
if not diagnostic.get("suggested_action") or "migrate" not in diagnostic["suggested_action"]:
    raise SystemExit(f"{os.environ['COMMAND_NAME']} rejection has no migration action")
if not diagnostic.get("primary", {}).get("path"):
    raise SystemExit(f"{os.environ['COMMAND_NAME']} rejection has no source path")
PY
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

case "$version" in
  0.1.*)
    echo "checking 0.1.x dual-source dogfood acceptance for $version"
    run_focused_test show_does_not_mark_invalid_runner_metadata_as_verified
    cargo test --locked --quiet -p mitase-workspace current_release_policy_keeps_dual_source_during_0_1_x
    run_focused_test mitase_authoring_v2_preserves_the_pre_migration_canonical_graph
    run_focused_test self_hosted_config_preserves_the_exact_artifact_resolution_baseline
    run_focused_test ugoite_current_v2_corpus_covers_all_output_contracts
    run_focused_test cli_help_contract_fixture_matches_the_current_read_only_surface
    ;;
  0.2.*)
    echo "checking 0.2.x single-source cutover acceptance for $version"
    assert_v1_rejected \
      check \
      "$tmp_dir/check.json" \
      "$tmp_dir/check.stderr" \
      check fixtures/v1/valid-web-app --format json
    assert_v1_rejected \
      validate \
      "$tmp_dir/validate.json" \
      "$tmp_dir/validate.stderr" \
      validate workspace fixtures/v1/valid-web-app --format json

    run_mitase migrate fixtures/v1/valid-web-app/spec/feature.yaml --stdout >"$tmp_dir/migrated.yaml"
    OUTPUT_PATH="$tmp_dir/migrated.yaml" python3 - <<'PY'
import os
from pathlib import Path

source = Path(os.environ["OUTPUT_PATH"]).read_text(encoding="utf-8")
if "schema: mitase/authoring/v2" not in source:
    raise SystemExit("migrate did not emit mitase/authoring/v2 source")
PY

    cargo test --locked --quiet -p mitase-workspace release_policy_switches_to_v2_only_for_the_0_2_line
    run_focused_test mitase_authoring_v2_preserves_the_pre_migration_canonical_graph
    run_focused_test self_hosted_config_preserves_the_exact_artifact_resolution_baseline
    python3 scripts/ci/check-architecture.py
    run_boundary_test
    ;;
  *)
    echo "unsupported release line for acceptance gate: $version" >&2
    exit 1
    ;;
esac

echo "release-line acceptance passed for $version"
