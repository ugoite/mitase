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

assert_normalize_contract() {
  local source="$1"
  local json_path="$2"
  local yaml_path="$3"

  run_mitase normalize "$source" --stdout --format json >"$json_path"
  run_mitase normalize "$source" --stdout --format yaml >"$yaml_path"

  SOURCE="$source" JSON_PATH="$json_path" YAML_PATH="$yaml_path" python3 - <<'PY'
import json
import os
from pathlib import Path

payload = json.loads(Path(os.environ["JSON_PATH"]).read_text(encoding="utf-8"))
if payload.get("contract_version") != "mitase/normalization-result/v1":
    raise SystemExit(f"{os.environ['SOURCE']} did not emit the normalization contract")
semantic = payload.get("semantic")
if not isinstance(semantic, dict) or "kind" not in semantic:
    raise SystemExit(f"{os.environ['SOURCE']} normalize output has no semantic payload")
if "schema" in semantic:
    raise SystemExit(f"{os.environ['SOURCE']} semantic payload retains a schema field")
if "target_schema" in payload.get("provenance", {}):
    raise SystemExit(f"{os.environ['SOURCE']} provenance retains target_schema")
if payload.get("provenance", {}).get("source_schema") != "mitase/authoring/v2":
    raise SystemExit(f"{os.environ['SOURCE']} provenance has an unexpected source schema")
if "document" in payload:
    raise SystemExit(f"{os.environ['SOURCE']} normalize output retains the document key")

text = Path(os.environ["YAML_PATH"]).read_text(encoding="utf-8")
if "contract_version: mitase/normalization-result/v1" not in text:
    raise SystemExit(f"{os.environ['SOURCE']} yaml output lacks the contract version")
if "mitase/spec/v1" in text:
    raise SystemExit(f"{os.environ['SOURCE']} yaml output contains the legacy schema")
PY
}

tmp_dir="$(mktemp -d)"
trap 'rm -rf "$tmp_dir"' EXIT

case "$version" in
  0.2.*)
    echo "checking 0.2.x semantic acceptance for $version"
    run_mitase check .

    cp -R fixtures/first-run-short "$tmp_dir/oversized-authoring"
    git -C "$tmp_dir/oversized-authoring" init --quiet
    git -C "$tmp_dir/oversized-authoring" add .
    git -C "$tmp_dir/oversized-authoring" \
      -c user.name="Mitase release acceptance" \
      -c user.email="release-acceptance@mitase.invalid" \
      commit --quiet -m "Prepare authoring budget acceptance fixture"
    OVERSIZED_SOURCE="$tmp_dir/oversized-authoring/docs/mitase/requirement.yaml" python3 - <<'PY'
import os
from pathlib import Path

source = Path(os.environ["OVERSIZED_SOURCE"])
with source.open("a", encoding="utf-8") as handle:
    for index in range(1001):
        handle.write(f"# release acceptance size guard {index}\n")
PY

    if run_mitase check "$tmp_dir/oversized-authoring" --format json >"$tmp_dir/oversized.json" 2>"$tmp_dir/oversized.stderr"; then
      echo "0.2.x release acceptance unexpectedly accepted an oversized authoring document" >&2
      exit 1
    fi
    OUTPUT_PATH="$tmp_dir/oversized.json" python3 - <<'PY'
import json
import os
from pathlib import Path

payload = json.loads(Path(os.environ["OUTPUT_PATH"]).read_text(encoding="utf-8"))
diagnostics = payload.get("diagnostics", [])
matching = [item for item in diagnostics if item.get("code") == "MITASE-AUTHORING-005"]
if len(matching) != 1:
    raise SystemExit("0.2.x release acceptance did not report exactly one MITASE-AUTHORING-005")
diagnostic = matching[0]
if diagnostic.get("severity") != "error":
    raise SystemExit("oversized authoring diagnostic is not an error")
evidence = {item["kind"]: item["value"] for item in diagnostic.get("evidence", [])}
if evidence.get("configured-limit") != "1000" or int(evidence.get("actual", "0")) <= 1000:
    raise SystemExit("oversized authoring diagnostic does not prove the default 1000-line limit")
if not diagnostic.get("primary", {}).get("path", "").endswith("docs/mitase/requirement.yaml"):
    raise SystemExit("oversized authoring diagnostic points at an unexpected source path")
PY

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

    if run_mitase migrate fixtures/v1/valid-web-app/spec/feature.yaml >"$tmp_dir/migrate.stdout" 2>"$tmp_dir/migrate.stderr"; then
      echo "migrate unexpectedly wrote without --stdout" >&2
      exit 1
    fi
    run_mitase migrate fixtures/v1/valid-web-app/spec/feature.yaml --stdout >"$tmp_dir/migrated.yaml"
    OUTPUT_PATH="$tmp_dir/migrated.yaml" python3 - <<'PY'
import os
from pathlib import Path

source = Path(os.environ["OUTPUT_PATH"]).read_text(encoding="utf-8")
if "schema: mitase/authoring/v2" not in source:
    raise SystemExit("migrate did not emit mitase/authoring/v2 source")
PY

    assert_normalize_contract \
      fixtures/first-run-short/docs/mitase/requirement.yaml \
      "$tmp_dir/normalized.json" \
      "$tmp_dir/normalized.yaml"

    run_focused_test mitase_authoring_v2_preserves_v022_semantic_graph
    run_focused_test self_hosted_config_preserves_the_exact_artifact_resolution_baseline
    run_focused_test report_facets_projects_explicit_facets_with_declared_verification
    run_focused_test report_facets_keeps_operation_style_features_valid
    run_mitase report facets FEAT-FACET-001 fixtures/acceptance/facet-oriented-v2 --format json >"$tmp_dir/facets.json"
    run_mitase report facets FEAT-OPS-001 fixtures/acceptance/ugoite-current-ops-v2 --format json >"$tmp_dir/ugoite-facets.json"
    OUTPUT_PATH="$tmp_dir/facets.json" python3 - <<'PY'
import json
import os
from pathlib import Path

report = json.loads(Path(os.environ["OUTPUT_PATH"]).read_text(encoding="utf-8"))
if report.get("contract_version") != "mitase/facet-projection-report/v1":
    raise SystemExit("facet report has an unexpected contract version")
criteria = {entry["criterion"]: entry for entry in report.get("criteria", [])}
creation = criteria.get("REQ-FACET-001#criterion.creation", {}).get("facets", [])
facets = sorted(row["facet"] for row in creation)
if facets != ["core", "frontend", "mcp", "weird-project-specific-name"]:
    raise SystemExit(f"facet report dropped an opaque facet: {facets}")
if any(row["declared_verification"]["status"] != "verified" for row in creation):
    raise SystemExit("facet report lost declared verification")
PY
    python3 scripts/ci/check-architecture.py
    run_boundary_test
    run_focused_test cli_help_contract_fixture_matches_the_current_read_only_surface
    ;;
  *)
    echo "unsupported release line for acceptance gate: $version" >&2
    exit 1
    ;;
esac

echo "release-line acceptance passed for $version"
