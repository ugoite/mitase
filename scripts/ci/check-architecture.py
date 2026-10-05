#!/usr/bin/env python3
"""Check the canonical production-crate boundary and dependency DAG."""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]

# Keep this table aligned with the production-crate responsibility table in
# docs/understand/model/v1-architecture.md. These are architectural edges,
# not a list of incidental imports from individual source files.
ALLOWED_INTERNAL_DEPENDENCIES = {
    "mitase": {
        "mitase-authoring",
        "mitase-code-intel",
        "mitase-diagnostics",
        "mitase-inventory",
        "mitase-migration",
        "mitase-project-model",
        "mitase-spec-model",
        "mitase-validation",
        "mitase-workspace",
    },
    "mitase-code-intel": {"mitase-spec-model"},
    "mitase-authoring": {"mitase-spec-model"},
    "mitase-migration": {"mitase-authoring", "mitase-spec-model"},
    "mitase-diagnostics": {"mitase-spec-model"},
    "mitase-inventory": {"mitase-project-model", "mitase-spec-model"},
    "mitase-project-model": {"mitase-spec-model"},
    "mitase-spec-model": set(),
    "mitase-validation": {
        "mitase-diagnostics",
        "mitase-inventory",
        "mitase-project-model",
        "mitase-spec-model",
        "mitase-workspace",
    },
    "mitase-workspace": {
        "mitase-authoring",
        "mitase-code-intel",
        "mitase-inventory",
        "mitase-project-model",
        "mitase-spec-model",
    },
}


class ArchitectureError(Exception):
    """A repository architecture invariant was violated."""


def metadata() -> dict:
    result = subprocess.run(
        ["cargo", "metadata", "--locked", "--no-deps", "--format-version", "1"],
        cwd=REPO_ROOT,
        check=False,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        detail = result.stderr.strip() or result.stdout.strip() or "unknown cargo metadata error"
        raise ArchitectureError(f"cargo metadata failed: {detail}")
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError as error:
        raise ArchitectureError(f"cargo metadata returned invalid JSON: {error}") from error


def relative(path: str | Path) -> str:
    return Path(path).resolve().relative_to(REPO_ROOT).as_posix()


# The removed v1 canonical model must never re-enter normal Rust code.
# Only the migration crate may name it; every other production surface and
# test must use the semantic representation or the legacy-only migration AST.
LEGACY_FORBIDDEN_IDENTIFIERS = ("SPEC_SCHEMA", "SpecSourcePolicy", "SpecDocument")

LEGACY_SCHEMA_LITERAL = "mitase/spec/v1"

# Production files allowed to name the legacy schema, with the only permitted
# line shapes. The workspace loader recognizes the legacy schema solely to
# route MITASE-SOURCE-001 migration guidance; it never parses that shape.
LEGACY_LITERAL_ALLOWLIST = {
    "crates/mitase-workspace/src/lib.rs": (
        "LEGACY_SOURCE_SCHEMA",
        "legacy mitase/spec/v1 source is not accepted",
    ),
}


def production_lines(path: Path) -> list[str]:
    """Return the non-test lines of a Rust source file."""
    lines = path.read_text(encoding="utf-8").splitlines()
    production = []
    for line in lines:
        if line.strip() == "#[cfg(test)]":
            break
        production.append(line)
    return production


def rust_sources(root: Path) -> list[Path]:
    return sorted(root.rglob("*.rs"))


def check_legacy_leakage(errors: list[str]) -> None:
    identifier_pattern = re.compile(
        r"\b(?:" + "|".join(LEGACY_FORBIDDEN_IDENTIFIERS) + r")\b"
    )
    for source in (
        rust_sources(REPO_ROOT / "src")
        + rust_sources(REPO_ROOT / "crates")
        + rust_sources(REPO_ROOT / "tests")
    ):
        if "mitase-migration" in source.parts:
            continue
        relative_path = relative(source)
        if relative_path.startswith("tests/"):
            lines = source.read_text(encoding="utf-8").splitlines()
            for number, line in enumerate(lines, start=1):
                if identifier_pattern.search(line):
                    errors.append(
                        f"{relative_path}:{number} reintroduces the removed canonical model"
                    )
            continue
        for number, line in enumerate(production_lines(source), start=1):
            if identifier_pattern.search(line):
                errors.append(
                    f"{relative_path}:{number} reintroduces the removed canonical model"
                )
            if LEGACY_SCHEMA_LITERAL in line:
                allowed = LEGACY_LITERAL_ALLOWLIST.get(relative_path, ())
                if not any(shape in line for shape in allowed):
                    errors.append(
                        f"{relative_path}:{number} names the legacy schema outside the allowlist"
                    )


def check() -> None:
    data = metadata()
    packages = {package["name"]: package for package in data["packages"]}
    expected_packages = set(ALLOWED_INTERNAL_DEPENDENCIES)
    errors: list[str] = []

    missing = sorted(expected_packages - packages.keys())
    unexpected = sorted(packages.keys() - expected_packages)
    if missing:
        errors.append(f"canonical production packages are missing: {', '.join(missing)}")
    if unexpected:
        errors.append(f"unclassified workspace packages: {', '.join(unexpected)}")

    metadata_manifests = {
        Path(package["manifest_path"]).resolve() for package in packages.values()
    }
    for manifest in sorted((REPO_ROOT / "crates").rglob("Cargo.toml")):
        if manifest.resolve() not in metadata_manifests:
            errors.append(
                "crate manifest is not a canonical workspace package: "
                f"{relative(manifest)}"
            )

    for name, package in packages.items():
        if name not in ALLOWED_INTERNAL_DEPENDENCIES:
            continue
        dependencies = {
            dependency["name"]
            for dependency in package["dependencies"]
            if dependency.get("source") is None and dependency["name"] in packages
        }
        disallowed = sorted(dependencies - ALLOWED_INTERNAL_DEPENDENCIES[name])
        if disallowed:
            errors.append(
                f"{name} has disallowed internal dependencies: {', '.join(disallowed)}"
            )

    check_legacy_leakage(errors)

    if errors:
        raise ArchitectureError("\n".join(f"- {error}" for error in errors))


def main() -> int:
    try:
        check()
    except ArchitectureError as error:
        print(f"Architecture boundary check failed:\n{error}", file=sys.stderr)
        return 1
    print("Architecture boundary check passed.")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
