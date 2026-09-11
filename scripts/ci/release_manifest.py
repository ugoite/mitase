#!/usr/bin/env python3
"""Build and validate the public Mitase release manifest."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

from release_candidate import CandidateError, digest_file, validate_manifest


SCHEMA = "mitase/release-manifest/v1"
SUPPORTED_TARGETS = {
    "aarch64-apple-darwin",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "x86_64-unknown-linux-gnu",
}
VERSION_PATTERN = re.compile(r"^v\d+\.\d+\.\d+(?:-(?:alpha|beta)\.\d+)?$")


def canonical_bytes(payload: dict[str, object]) -> bytes:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def build_manifest(candidate: dict[str, object]) -> dict[str, object]:
    validate_manifest(candidate)
    version = candidate["version"]
    assert isinstance(version, str)
    artifacts = candidate["artifacts"]
    assert isinstance(artifacts, list)

    prefix = f"mitase-{version}-"
    selected: list[dict[str, str]] = []
    for artifact in artifacts:
        assert isinstance(artifact, dict)
        name = artifact["name"]
        digest = artifact["sha256"]
        assert isinstance(name, str)
        assert isinstance(digest, str)
        if not name.startswith(prefix) or not name.endswith(".tar.gz"):
            continue
        target = name[len(prefix) : -len(".tar.gz")]
        if target not in SUPPORTED_TARGETS:
            raise CandidateError(f"unsupported release target in archive name: {name}")
        selected.append({"name": name, "target": target, "sha256": digest})

    selected.sort(key=lambda artifact: artifact["name"])
    if {artifact["target"] for artifact in selected} != SUPPORTED_TARGETS:
        raise CandidateError(
            "release manifest must contain exactly the four supported target archives"
        )

    return {
        "schema": SCHEMA,
        "version": version,
        "source_sha": candidate["source_sha"],
        "candidate_id": candidate["candidate_id"],
        "artifacts": selected,
    }


def load_json(path: Path) -> dict[str, object]:
    try:
        payload = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CandidateError(f"unable to read release manifest: {error}") from error
    if not isinstance(payload, dict):
        raise CandidateError("release manifest must be a JSON object")
    return payload


def validate_release_manifest(
    manifest: dict[str, object], candidate: dict[str, object], artifact_root: Path
) -> None:
    expected = build_manifest(candidate)
    if manifest != expected:
        raise CandidateError("release manifest does not match the candidate manifest")

    artifacts = manifest["artifacts"]
    assert isinstance(artifacts, list)
    for artifact in artifacts:
        assert isinstance(artifact, dict)
        name = artifact["name"]
        digest = artifact["sha256"]
        assert isinstance(name, str)
        assert isinstance(digest, str)
        if digest_file(artifact_root / name) != digest:
            raise CandidateError(f"release archive digest does not match manifest: {name}")


def parser() -> argparse.ArgumentParser:
    command_parser = argparse.ArgumentParser(description=__doc__)
    commands = command_parser.add_subparsers(dest="command", required=True)

    build = commands.add_parser("build", help="build a public release manifest")
    build.add_argument("--candidate-manifest", type=Path, required=True)
    build.add_argument("--output", type=Path, required=True)
    build.set_defaults(handler=build_command)

    validate = commands.add_parser("validate", help="validate a public release manifest")
    validate.add_argument("--candidate-manifest", type=Path, required=True)
    validate.add_argument("--manifest", type=Path, required=True)
    validate.add_argument("--artifact-root", type=Path, required=True)
    validate.set_defaults(handler=validate_command)
    return command_parser


def build_command(arguments: argparse.Namespace) -> None:
    manifest = build_manifest(load_json(arguments.candidate_manifest))
    arguments.output.parent.mkdir(parents=True, exist_ok=True)
    arguments.output.write_bytes(canonical_bytes(manifest))


def validate_command(arguments: argparse.Namespace) -> None:
    candidate = load_json(arguments.candidate_manifest)
    manifest = load_json(arguments.manifest)
    if manifest.get("schema") != SCHEMA:
        raise CandidateError(f"release manifest schema must be {SCHEMA}")
    version = manifest.get("version")
    if not isinstance(version, str) or not VERSION_PATTERN.fullmatch(version):
        raise CandidateError("release manifest version is malformed")
    if arguments.manifest.read_bytes() != canonical_bytes(manifest):
        raise CandidateError("release manifest is not in canonical encoding")
    validate_release_manifest(manifest, candidate, arguments.artifact_root)


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        arguments.handler(arguments)
    except CandidateError as error:
        print(f"release manifest error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
