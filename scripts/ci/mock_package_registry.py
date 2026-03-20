#!/usr/bin/env python3
# FEAT-INSTALL-001

from __future__ import annotations

import argparse
import hashlib
import io
import json
import tarfile
import threading
import zipfile
from dataclasses import dataclass
from http import HTTPStatus
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from typing import Dict


TARGETS = {
    "x86_64-unknown-linux-gnu": "syu",
    "x86_64-apple-darwin": "syu",
    "aarch64-apple-darwin": "syu",
    "x86_64-pc-windows-msvc": "syu.exe",
}

TAG_SETS = {
    "prerelease": ["v0.0.1-alpha.2", "v0.0.1-alpha.3", "v0.0.2-beta.1"],
    "mixed": ["v0.0.1-alpha.2", "v0.0.1-alpha.3", "v0.0.2-beta.1", "v0.0.2"],
}


@dataclass(frozen=True)
class Artifact:
    archive_name: str
    archive_bytes: bytes
    archive_digest: str
    checksum_bytes: bytes
    checksum_digest: str
    manifest_bytes: bytes


def build_archive(version: str, target: str, binary_name: str) -> bytes:
    payload = f"mock syu {version} {target}\n".encode()
    buffer = io.BytesIO()
    if binary_name.endswith(".exe"):
        with zipfile.ZipFile(buffer, "w", compression=zipfile.ZIP_DEFLATED) as archive:
            archive.writestr(binary_name, payload)
    else:
        with tarfile.open(fileobj=buffer, mode="w:gz") as archive:
            info = tarfile.TarInfo(name=binary_name)
            info.size = len(payload)
            info.mode = 0o755
            archive.addfile(info, io.BytesIO(payload))
    return buffer.getvalue()


def sha256_digest(content: bytes) -> str:
    return f"sha256:{hashlib.sha256(content).hexdigest()}"


def build_artifacts(
    package_repository: str, mode: str, target_filter: str | None
) -> Dict[str, Artifact]:
    artifacts: Dict[str, Artifact] = {}
    for version in TAG_SETS[mode]:
        for target, binary_name in TARGETS.items():
            if target_filter is not None and target != target_filter:
                continue
            archive_name = (
                f"syu-{target}.zip" if target.endswith("windows-msvc") else f"syu-{target}.tar.gz"
            )
            archive_bytes = build_archive(version, target, binary_name)
            archive_digest = sha256_digest(archive_bytes)
            checksum_bytes = f"{archive_digest.split(':', 1)[1]}  {archive_name}\n".encode()
            checksum_digest = sha256_digest(checksum_bytes)
            manifest = {
                "schemaVersion": 2,
                "mediaType": "application/vnd.oci.image.manifest.v1+json",
                "config": {
                    "mediaType": "application/vnd.syu.config.v1+json",
                    "digest": sha256_digest(b"{}"),
                    "size": 2,
                },
                "layers": [
                    {
                        "mediaType": "application/vnd.syu.archive.layer.v1",
                        "digest": archive_digest,
                        "size": len(archive_bytes),
                        "annotations": {"org.opencontainers.image.title": archive_name},
                    },
                    {
                        "mediaType": "text/plain",
                        "digest": checksum_digest,
                        "size": len(checksum_bytes),
                        "annotations": {
                            "org.opencontainers.image.title": f"{archive_name}.sha256"
                        },
                    },
                ],
                "annotations": {
                    "org.opencontainers.image.source": "https://github.com/ugoite/syu",
                    "org.opencontainers.image.version": version,
                    "org.opencontainers.image.title": package_repository,
                },
            }
            tag = f"{version}__{target}"
            artifacts[tag] = Artifact(
                archive_name=archive_name,
                archive_bytes=archive_bytes,
                archive_digest=archive_digest,
                checksum_bytes=checksum_bytes,
                checksum_digest=checksum_digest,
                manifest_bytes=json.dumps(manifest).encode(),
            )
    return artifacts


def build_handler(package_repository: str, mode: str, target_filter: str | None):
    repository_prefix = f"/v2/{package_repository}"
    artifacts: Dict[str, Artifact] | None = None
    artifact_lock = threading.Lock()

    def get_artifacts() -> Dict[str, Artifact]:
        nonlocal artifacts
        if artifacts is None:
            with artifact_lock:
                if artifacts is None:
                    artifacts = build_artifacts(package_repository, mode, target_filter)
        return artifacts

    class RegistryHandler(BaseHTTPRequestHandler):
        def do_GET(self) -> None:  # noqa: N802
            current_artifacts = get_artifacts()

            if self.path.startswith("/token?scope=repository:"):
                self.respond_json({"token": "mock-token"})
                return

            if self.path == f"{repository_prefix}/tags/list":
                self.respond_json({"name": package_repository, "tags": sorted(current_artifacts)})
                return

            if self.path.startswith(f"{repository_prefix}/manifests/"):
                tag = self.path.removeprefix(f"{repository_prefix}/manifests/")
                artifact = current_artifacts.get(tag)
                if artifact is None:
                    self.send_error(HTTPStatus.NOT_FOUND)
                    return
                self.respond_bytes(
                    artifact.manifest_bytes,
                    content_type="application/vnd.oci.image.manifest.v1+json",
                )
                return

            if self.path.startswith(f"{repository_prefix}/blobs/"):
                digest = self.path.removeprefix(f"{repository_prefix}/blobs/")
                for artifact in current_artifacts.values():
                    if digest == artifact.archive_digest:
                        self.respond_bytes(artifact.archive_bytes, "application/octet-stream")
                        return
                    if digest == artifact.checksum_digest:
                        self.respond_bytes(artifact.checksum_bytes, "text/plain")
                        return
                self.send_error(HTTPStatus.NOT_FOUND)
                return

            self.send_error(HTTPStatus.NOT_FOUND)

        def log_message(self, format: str, *args) -> None:  # noqa: A003
            return

        def respond_json(self, payload: dict) -> None:
            self.respond_bytes(json.dumps(payload).encode(), "application/json")

        def respond_bytes(self, content: bytes, content_type: str) -> None:
            self.send_response(HTTPStatus.OK)
            self.send_header("Content-Type", content_type)
            self.send_header("Content-Length", str(len(content)))
            self.end_headers()
            self.wfile.write(content)

    return RegistryHandler


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--package-repository", default="test/syu")
    parser.add_argument("--mode", choices=sorted(TAG_SETS), default="mixed")
    parser.add_argument("--port", type=int)
    parser.add_argument("--target")
    args = parser.parse_args()

    handler = build_handler(args.package_repository, args.mode, args.target)
    server = ThreadingHTTPServer(("127.0.0.1", args.port or 0), handler)

    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    finally:
        server.server_close()


if __name__ == "__main__":
    main()
