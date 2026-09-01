#!/usr/bin/env python3
"""Build and validate the immutable Mitase release-candidate manifest."""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from pathlib import Path


SCHEMA = "mitase/release-candidate/v1"
VERSION_PATTERN = re.compile(r"^v\d+\.\d+\.\d+(?:-(?:alpha|beta)\.\d+)?$")
GIT_SHA_PATTERN = re.compile(r"^[0-9a-f]{40}$")
DIGEST_PATTERN = re.compile(r"^sha256:[0-9a-f]{64}$")
ARTIFACT_NAME_PATTERN = re.compile(r"^[A-Za-z0-9][A-Za-z0-9._-]*$")


class CandidateError(ValueError):
    """A candidate input or manifest is invalid."""


def validate_version(version: str) -> None:
    if not VERSION_PATTERN.fullmatch(version):
        raise CandidateError(
            "version must be v<major>.<minor>.<patch> with an optional "
            "alpha or beta prerelease"
        )


def validate_source_sha(source_sha: str, repository: Path) -> None:
    if not GIT_SHA_PATTERN.fullmatch(source_sha):
        raise CandidateError("source SHA must be exactly 40 lowercase hexadecimal characters")

    try:
        result = subprocess.run(
            ["git", "-C", str(repository), "cat-file", "-e", f"{source_sha}^{{commit}}"],
            check=False,
            capture_output=True,
            text=True,
        )
    except OSError as error:
        raise CandidateError(f"unable to inspect source Git revision: {error}") from error

    if result.returncode != 0:
        raise CandidateError(f"source SHA does not resolve to a commit: {source_sha}")


def validate_artifact_name(name: str) -> None:
    if not ARTIFACT_NAME_PATTERN.fullmatch(name):
        raise CandidateError(
            f"artifact name must be a portable file name containing only letters, "
            f"digits, '.', '_' or '-': {name!r}"
        )


def parse_artifact_argument(argument: str) -> tuple[str, Path]:
    name, separator, path = argument.partition("=")
    if not separator or not name or not path:
        raise CandidateError("artifact must use the NAME=PATH form")
    validate_artifact_name(name)
    return name, Path(path)


def digest_file(path: Path) -> str:
    if not path.is_file():
        raise CandidateError(f"artifact path is not a file: {path}")

    digest = hashlib.sha256()
    with path.open("rb") as artifact:
        for chunk in iter(lambda: artifact.read(1024 * 1024), b""):
            digest.update(chunk)
    value = f"sha256:{digest.hexdigest()}"
    if not DIGEST_PATTERN.fullmatch(value):
        raise CandidateError(f"computed artifact digest is malformed: {value}")
    return value


def canonical_bytes(payload: dict[str, object]) -> bytes:
    return (
        json.dumps(payload, ensure_ascii=False, sort_keys=True, separators=(",", ":")) + "\n"
    ).encode("utf-8")


def candidate_id(payload: dict[str, object]) -> str:
    return f"sha256:{hashlib.sha256(canonical_bytes(payload)).hexdigest()}"


def validate_manifest(manifest: object) -> dict[str, object]:
    if not isinstance(manifest, dict):
        raise CandidateError("candidate manifest must be a JSON object")

    expected_keys = {"schema", "version", "source_sha", "artifacts", "candidate_id"}
    if set(manifest) != expected_keys:
        raise CandidateError("candidate manifest contains missing or unknown fields")
    if manifest["schema"] != SCHEMA:
        raise CandidateError(f"candidate manifest schema must be {SCHEMA}")

    version = manifest["version"]
    source_sha = manifest["source_sha"]
    candidate = manifest["candidate_id"]
    artifacts = manifest["artifacts"]
    if not isinstance(version, str):
        raise CandidateError("candidate manifest version must be a string")
    validate_version(version)
    if not isinstance(source_sha, str) or not GIT_SHA_PATTERN.fullmatch(source_sha):
        raise CandidateError("candidate manifest source_sha must be a 40-character lowercase Git SHA")
    if not isinstance(candidate, str) or not DIGEST_PATTERN.fullmatch(candidate):
        raise CandidateError("candidate_id must be a sha256: lowercase hexadecimal digest")
    if not isinstance(artifacts, list) or not artifacts:
        raise CandidateError("candidate manifest artifacts must be a non-empty array")

    normalized_artifacts: list[dict[str, str]] = []
    names: set[str] = set()
    for artifact in artifacts:
        if not isinstance(artifact, dict) or set(artifact) != {"name", "sha256"}:
            raise CandidateError("each artifact must contain only name and sha256")
        name = artifact["name"]
        digest = artifact["sha256"]
        if not isinstance(name, str):
            raise CandidateError("artifact name must be a string")
        validate_artifact_name(name)
        if name in names:
            raise CandidateError(f"duplicate artifact name: {name}")
        names.add(name)
        if not isinstance(digest, str) or not DIGEST_PATTERN.fullmatch(digest):
            raise CandidateError(f"artifact {name} has a malformed sha256 digest")
        normalized_artifacts.append({"name": name, "sha256": digest})

    identity = {
        "schema": SCHEMA,
        "version": version,
        "source_sha": source_sha,
        "artifacts": sorted(normalized_artifacts, key=lambda artifact: artifact["name"]),
    }
    if candidate != candidate_id(identity):
        raise CandidateError("candidate_id does not match the canonical manifest identity")

    return manifest


def build_manifest(
    version: str, source_sha: str, repository: Path, artifact_arguments: list[str]
) -> dict[str, object]:
    validate_version(version)
    validate_source_sha(source_sha, repository)

    artifacts: list[dict[str, str]] = []
    names: set[str] = set()
    for argument in artifact_arguments:
        name, path = parse_artifact_argument(argument)
        if name in names:
            raise CandidateError(f"duplicate artifact name: {name}")
        names.add(name)
        artifacts.append({"name": name, "sha256": digest_file(path)})
    artifacts.sort(key=lambda artifact: artifact["name"])

    identity = {
        "schema": SCHEMA,
        "version": version,
        "source_sha": source_sha,
        "artifacts": artifacts,
    }
    manifest = {**identity, "candidate_id": candidate_id(identity)}
    validate_manifest(manifest)
    return manifest


def build_command(arguments: argparse.Namespace) -> None:
    manifest = build_manifest(
        arguments.version,
        arguments.source_sha,
        arguments.repository.resolve(),
        arguments.artifact,
    )
    output = arguments.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(canonical_bytes(manifest))


def validate_command(arguments: argparse.Namespace) -> None:
    try:
        manifest = json.loads(arguments.manifest.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise CandidateError(f"unable to read candidate manifest: {error}") from error
    validate_manifest(manifest)


def parser() -> argparse.ArgumentParser:
    command_parser = argparse.ArgumentParser(description=__doc__)
    commands = command_parser.add_subparsers(dest="command", required=True)

    build = commands.add_parser("build", help="build a canonical candidate manifest")
    build.add_argument("--version", required=True)
    build.add_argument("--source-sha", required=True)
    build.add_argument("--repository", type=Path, default=Path("."))
    build.add_argument("--output", type=Path, required=True)
    build.add_argument("--artifact", action="append", required=True, metavar="NAME=PATH")
    build.set_defaults(handler=build_command)

    validate = commands.add_parser("validate", help="validate a candidate manifest")
    validate.add_argument("--manifest", type=Path, required=True)
    validate.set_defaults(handler=validate_command)
    return command_parser


def main(argv: list[str] | None = None) -> int:
    arguments = parser().parse_args(argv)
    try:
        arguments.handler(arguments)
    except CandidateError as error:
        print(f"candidate manifest error: {error}", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
