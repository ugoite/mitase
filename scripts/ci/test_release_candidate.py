from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).with_name("release_candidate.py")
sys.path.insert(0, str(SCRIPT.parent))

from release_candidate import CandidateError, candidate_id, validate_manifest  # noqa: E402


class ReleaseCandidateTests(unittest.TestCase):
    def setUp(self) -> None:
        self.temp_dir = tempfile.TemporaryDirectory()
        self.repository = Path(self.temp_dir.name)
        subprocess.run(["git", "init", "-q", str(self.repository)], check=True)
        subprocess.run(
            ["git", "-C", str(self.repository), "config", "user.email", "test@example.invalid"],
            check=True,
        )
        subprocess.run(
            ["git", "-C", str(self.repository), "config", "user.name", "Release Candidate Test"],
            check=True,
        )
        (self.repository / "mitase-x86_64-unknown-linux-gnu.tar.gz").write_bytes(b"archive-a\n")
        (self.repository / "mitase-aarch64-apple-darwin.tar.gz").write_bytes(b"archive-b\n")
        subprocess.run(["git", "-C", str(self.repository), "add", "."], check=True)
        subprocess.run(
            ["git", "-C", str(self.repository), "commit", "-qm", "fixture"], check=True
        )
        self.source_sha = subprocess.check_output(
            ["git", "-C", str(self.repository), "rev-parse", "HEAD"], text=True
        ).strip()

    def tearDown(self) -> None:
        self.temp_dir.cleanup()

    def run_cli(self, *arguments: str) -> subprocess.CompletedProcess[str]:
        return subprocess.run(
            [sys.executable, str(SCRIPT), *arguments],
            cwd=self.repository,
            capture_output=True,
            text=True,
        )

    def test_build_is_deterministic_and_validates_against_the_manifest(self) -> None:
        first = self.repository / "candidate-a.json"
        second = self.repository / "candidate-b.json"
        common = [
            "build",
            "--version",
            "v0.1.0",
            "--source-sha",
            self.source_sha,
            "--repository",
            str(self.repository),
        ]
        first_result = self.run_cli(
            *common,
            "--output",
            str(first),
            "--artifact",
            "mitase-x86_64-unknown-linux-gnu.tar.gz="
            + str(self.repository / "mitase-x86_64-unknown-linux-gnu.tar.gz"),
            "--artifact",
            "mitase-aarch64-apple-darwin.tar.gz="
            + str(self.repository / "mitase-aarch64-apple-darwin.tar.gz"),
        )
        second_result = self.run_cli(
            *common,
            "--output",
            str(second),
            "--artifact",
            "mitase-aarch64-apple-darwin.tar.gz="
            + str(self.repository / "mitase-aarch64-apple-darwin.tar.gz"),
            "--artifact",
            "mitase-x86_64-unknown-linux-gnu.tar.gz="
            + str(self.repository / "mitase-x86_64-unknown-linux-gnu.tar.gz"),
        )

        self.assertEqual(first_result.returncode, 0, first_result.stderr)
        self.assertEqual(second_result.returncode, 0, second_result.stderr)
        self.assertEqual(first.read_bytes(), second.read_bytes())

        manifest = json.loads(first.read_text(encoding="utf-8"))
        self.assertEqual(manifest["schema"], "mitase/release-candidate/v1")
        self.assertEqual(manifest["source_sha"], self.source_sha)
        self.assertEqual(
            manifest["artifacts"][0]["name"], "mitase-aarch64-apple-darwin.tar.gz"
        )
        expected_digest = "sha256:" + hashlib.sha256(b"archive-b\n").hexdigest()
        self.assertEqual(manifest["artifacts"][0]["sha256"], expected_digest)
        self.assertEqual(self.run_cli("validate", "--manifest", str(first)).returncode, 0)

    def test_invalid_source_revision_is_rejected(self) -> None:
        result = self.run_cli(
            "build",
            "--version",
            "v0.1.0",
            "--source-sha",
            "0" * 40,
            "--repository",
            str(self.repository),
            "--output",
            str(self.repository / "candidate.json"),
            "--artifact",
            "archive.tar.gz=" + str(self.repository / "mitase-x86_64-unknown-linux-gnu.tar.gz"),
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("does not resolve to a commit", result.stderr)

    def test_duplicate_and_missing_artifacts_are_rejected(self) -> None:
        duplicate = self.run_cli(
            "build",
            "--version",
            "v0.1.0",
            "--source-sha",
            self.source_sha,
            "--repository",
            str(self.repository),
            "--output",
            str(self.repository / "candidate.json"),
            "--artifact",
            "archive.tar.gz=" + str(self.repository / "mitase-x86_64-unknown-linux-gnu.tar.gz"),
            "--artifact",
            "archive.tar.gz=" + str(self.repository / "mitase-aarch64-apple-darwin.tar.gz"),
        )
        missing = self.run_cli(
            "build",
            "--version",
            "v0.1.0",
            "--source-sha",
            self.source_sha,
            "--repository",
            str(self.repository),
            "--output",
            str(self.repository / "candidate.json"),
            "--artifact",
            "archive.tar.gz=" + str(self.repository / "missing.tar.gz"),
        )
        self.assertNotEqual(duplicate.returncode, 0)
        self.assertIn("duplicate artifact name", duplicate.stderr)
        self.assertNotEqual(missing.returncode, 0)
        self.assertIn("not a file", missing.stderr)

    def test_malformed_digest_and_candidate_id_are_rejected(self) -> None:
        identity = {
            "schema": "mitase/release-candidate/v1",
            "version": "v0.1.0",
            "source_sha": self.source_sha,
            "artifacts": [{"name": "archive.tar.gz", "sha256": "sha256:" + "a" * 64}],
        }
        manifest = {**identity, "candidate_id": candidate_id(identity)}
        manifest["artifacts"][0]["sha256"] = "sha256:NOT-A-DIGEST"
        with self.assertRaisesRegex(CandidateError, "malformed sha256 digest"):
            validate_manifest(manifest)

        manifest["artifacts"][0]["sha256"] = "sha256:" + "a" * 64
        manifest["candidate_id"] = "sha256:" + "b" * 64
        with self.assertRaisesRegex(CandidateError, "does not match"):
            validate_manifest(manifest)

    def test_noncanonical_manifest_encoding_is_rejected(self) -> None:
        identity = {
            "schema": "mitase/release-candidate/v1",
            "version": "v0.1.0",
            "source_sha": self.source_sha,
            "artifacts": [
                {"name": "a.tar.gz", "sha256": "sha256:" + "a" * 64},
                {"name": "b.tar.gz", "sha256": "sha256:" + "b" * 64},
            ],
        }
        manifest = {**identity, "candidate_id": candidate_id(identity)}
        manifest["artifacts"] = list(reversed(manifest["artifacts"]))
        path = self.repository / "candidate.json"
        path.write_text(json.dumps(manifest) + "\n", encoding="utf-8")
        result = self.run_cli("validate", "--manifest", str(path))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn("not in canonical encoding", result.stderr)

    def test_candidate_id_and_artifact_bytes_are_verified(self) -> None:
        artifact = self.repository / "artifact.tar.gz"
        artifact.write_bytes(b"candidate-bytes\n")
        manifest_path = self.repository / "candidate.json"
        build_result = self.run_cli(
            "build",
            "--version",
            "v0.1.0",
            "--source-sha",
            self.source_sha,
            "--repository",
            str(self.repository),
            "--output",
            str(manifest_path),
            "--artifact",
            "artifact.tar.gz=" + str(artifact),
        )
        self.assertEqual(build_result.returncode, 0, build_result.stderr)
        manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
        candidate = manifest["candidate_id"]

        valid = self.run_cli(
            "validate",
            "--manifest",
            str(manifest_path),
            "--candidate-id",
            candidate,
            "--artifact-root",
            str(self.repository),
        )
        self.assertEqual(valid.returncode, 0, valid.stderr)

        mismatch = self.run_cli(
            "validate",
            "--manifest",
            str(manifest_path),
            "--candidate-id",
            "sha256:" + "0" * 64,
        )
        self.assertNotEqual(mismatch.returncode, 0)
        self.assertIn("requested candidate", mismatch.stderr)

        artifact.write_bytes(b"tampered\n")
        tampered = self.run_cli(
            "validate",
            "--manifest",
            str(manifest_path),
            "--artifact-root",
            str(self.repository),
        )
        self.assertNotEqual(tampered.returncode, 0)
        self.assertIn("does not match manifest", tampered.stderr)


if __name__ == "__main__":
    unittest.main()
