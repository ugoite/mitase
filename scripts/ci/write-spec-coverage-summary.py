#!/usr/bin/env python3
# FEAT-QUALITY-001

from __future__ import annotations

import sys
from pathlib import Path

import yaml


def load_lcov(path: Path) -> dict[str, tuple[int, int]]:
    coverage: dict[str, tuple[int, int]] = {}
    current_path: str | None = None
    covered = 0
    total = 0

    for raw_line in path.read_text(encoding="utf-8").splitlines():
        if raw_line.startswith("SF:"):
            current_path = raw_line[3:]
            covered = 0
            total = 0
        elif raw_line.startswith("DA:"):
            _, payload = raw_line.split(":", 1)
            _, count = payload.split(",", 1)
            total += 1
            covered += int(int(count) > 0)
        elif raw_line == "end_of_record" and current_path is not None:
            coverage[current_path] = (covered, total)
            current_path = None

    return coverage


def percent_string(covered: int, total: int) -> str:
    if total == 0:
        return "0.0% (0/0)"
    return f"{covered * 100.0 / total:.1f}% ({covered}/{total})"


def resolve_workspace_path(repo_root: Path, raw_path: str) -> Path:
    path = Path(raw_path)
    return path if path.is_absolute() else repo_root / path


def anchor_item_id(value: str) -> str:
    return value.split("#", 1)[0]


def load_spec_items(
    repo_root: Path, spec_root: Path, kind: str, items_key: str
) -> dict[str, dict]:
    items_by_id: dict[str, dict] = {}
    for document_path in sorted(spec_root.rglob("*.yaml")):
        document = yaml.safe_load(document_path.read_text(encoding="utf-8"))
        if document.get("kind") != kind:
            continue
        for item in document.get(items_key, []):
            items_by_id[item["id"]] = item
    return items_by_id


def summarize_paths(
    repo_root: Path, lcov: dict[str, tuple[int, int]], paths: list[str]
) -> tuple[int, int, int]:
    covered = 0
    total = 0
    instrumented_paths = 0
    for path in sorted(set(paths)):
        stats = lcov.get(str(resolve_workspace_path(repo_root, path)))
        if stats is None:
            continue
        instrumented_paths += 1
        covered += stats[0]
        total += stats[1]
    return covered, total, instrumented_paths


def coverage_label(
    *,
    total_refs: int,
    rust_file_count: int,
    instrumented_paths: int,
    covered: int,
    total: int,
    empty_label: str,
) -> str:
    if total_refs == 0:
        return empty_label
    if rust_file_count == 0:
        return "no Rust files"
    if instrumented_paths == 0:
        return "not instrumented"
    return percent_string(covered, total)


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: write-spec-coverage-summary.py <lcov-path> <output-path>", file=sys.stderr)
        return 1

    lcov_path = Path(sys.argv[1]).resolve()
    output_path = Path(sys.argv[2]).resolve()
    repo_root = Path(__file__).resolve().parents[2]
    spec_root = repo_root / "docs" / "mitase"
    lcov = load_lcov(lcov_path)

    requirements = load_spec_items(repo_root, spec_root, "requirements", "requirements")
    features = load_spec_items(repo_root, spec_root, "features", "features")

    feature_details: dict[str, dict] = {}
    for feature_id, item in sorted(features.items()):
        implementation_targets = [
            target
            for binding in item.get("bindings", [])
            if binding.get("role") == "implementation"
            for target in binding.get("targets", [])
        ]
        implementation_refs = len(implementation_targets)
        rust_files = [
            target["path"]
            for target in implementation_targets
            if target.get("adapter") == "rust"
        ]
        linked_requirements = sorted(
            {
                anchor_item_id(reference)
                for binding in item.get("bindings", [])
                if binding.get("role") == "implementation"
                for reference in binding.get("satisfies", [])
            }
        )
        covered, total, instrumented_paths = summarize_paths(repo_root, lcov, rust_files)
        feature_details[feature_id] = {
            "linked_requirements": linked_requirements,
            "implementation_refs": implementation_refs,
            "rust_files": len(sorted(set(rust_files))),
            "rust_coverage": coverage_label(
                total_refs=implementation_refs,
                rust_file_count=len(sorted(set(rust_files))),
                instrumented_paths=instrumented_paths,
                covered=covered,
                total=total,
                empty_label="no implementation refs",
            ),
            "rust_paths": rust_files,
        }

    requirement_rows: list[str] = []
    for requirement_id, item in sorted(requirements.items()):
        verification_targets = [
            target
            for binding in item.get("bindings", [])
            if binding.get("role") == "verification"
            for target in binding.get("targets", [])
        ]
        test_refs = len(verification_targets)
        rust_test_files = [
            target["path"]
            for target in verification_targets
            if target.get("adapter") == "rust"
        ]
        test_covered, test_total, test_instrumented_paths = summarize_paths(
            repo_root, lcov, rust_test_files
        )

        linked_feature_ids = sorted(
            feature_id
            for feature_id, detail in feature_details.items()
            if requirement_id in detail["linked_requirements"]
        )
        linked_feature_paths: list[str] = []
        for feature_id in linked_feature_ids:
            linked_feature_paths.extend(feature_details.get(feature_id, {}).get("rust_paths", []))
        feature_covered, feature_total, feature_instrumented_paths = summarize_paths(
            repo_root, lcov, linked_feature_paths
        )
        linked_feature_rust_files = len(sorted(set(linked_feature_paths)))

        requirement_rows.append(
            "| {id} | {features} | {test_refs} | {test_coverage} | {feature_coverage} |".format(
                id=requirement_id,
                features=", ".join(linked_feature_ids) if linked_feature_ids else "—",
                test_refs=test_refs,
                test_coverage=coverage_label(
                    total_refs=test_refs,
                    rust_file_count=len(sorted(set(rust_test_files))),
                    instrumented_paths=test_instrumented_paths,
                    covered=test_covered,
                    total=test_total,
                    empty_label="no test refs",
                ),
                feature_coverage=(
                    "—"
                    if not linked_feature_ids
                    else coverage_label(
                        total_refs=len(linked_feature_ids),
                        rust_file_count=linked_feature_rust_files,
                        instrumented_paths=feature_instrumented_paths,
                        covered=feature_covered,
                        total=feature_total,
                        empty_label="no linked features",
                    )
                ),
            )
        )

    feature_rows = [
        "| {id} | {requirements} | {refs} | {files} | {coverage} |".format(
            id=feature_id,
            requirements=", ".join(detail["linked_requirements"]) if detail["linked_requirements"] else "—",
            refs=detail["implementation_refs"],
            files=detail["rust_files"],
            coverage=detail["rust_coverage"],
        )
        for feature_id, detail in sorted(feature_details.items())
    ]

    output = "\n".join(
        [
            "# Coverage by requirement and feature",
            "",
            "This report combines Rust line coverage from `cargo llvm-cov` with the current",
            "`mitase` requirement/feature trace graph so reviewers can inspect coverage in spec terms.",
            "",
            "## Requirements",
            "",
            "| Requirement | Linked features | Traced test refs | Rust test file coverage | Linked Rust implementation coverage |",
            "| --- | --- | ---: | ---: | ---: |",
            *requirement_rows,
            "",
            "## Features",
            "",
            "| Feature | Linked requirements | Implementation refs | Rust implementation files | Rust implementation coverage |",
            "| --- | --- | ---: | ---: | ---: |",
            *feature_rows,
            "",
        ]
    )

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_text(output, encoding="utf-8")
    print(output)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
