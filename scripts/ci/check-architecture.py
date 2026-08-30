#!/usr/bin/env python3
"""Check the canonical production-crate boundary and dependency DAG."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path


REPO_ROOT = Path(__file__).resolve().parents[2]

# Keep this table aligned with the production-crate responsibility table in
# docs/understand/model/v1-architecture.md. These are architectural edges,
# not a list of incidental imports from individual source files.
ALLOWED_INTERNAL_DEPENDENCIES = {
    "mitase": {
        "mitase-code-intel",
        "mitase-diagnostics",
        "mitase-inventory",
        "mitase-project-model",
        "mitase-spec-model",
        "mitase-validation",
        "mitase-workspace",
    },
    "mitase-code-intel": {"mitase-spec-model"},
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
