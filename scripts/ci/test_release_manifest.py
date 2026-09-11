from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]
SCRIPT = ROOT / "scripts/ci/release_manifest.py"
sys.path.insert(0, str(SCRIPT.parent))

from release_candidate import candidate_id  # noqa: E402
from release_manifest import CandidateError, build_manifest, canonical_bytes  # noqa: E402


TARGETS = sorted(
    (
        "x86_64-unknown-linux-gnu",
        "aarch64-unknown-linux-gnu",
        "x86_64-apple-darwin",
        "aarch64-apple-darwin",
    )
)


def candidate_manifest() -> dict[str, object]:
    identity = {
        "schema": "mitase/release-candidate/v1",
        "version": "v0.1.8",
        "source_sha": "a" * 40,
        "artifacts": [
            {
                "name": f"mitase-v0.1.8-{target}.tar.gz",
                "sha256": "sha256:" + hashlib.sha256(target.encode()).hexdigest(),
            }
            for target in TARGETS
        ],
    }
    return {**identity, "candidate_id": candidate_id(identity)}


class ReleaseManifestTests(unittest.TestCase):
    def test_public_manifest_contains_exact_supported_archives(self) -> None:
        manifest = build_manifest(candidate_manifest())

        self.assertEqual(manifest["schema"], "mitase/release-manifest/v1")
        self.assertEqual(manifest["version"], "v0.1.8")
        self.assertEqual(len(manifest["artifacts"]), 4)
        self.assertEqual(
            {artifact["target"] for artifact in manifest["artifacts"]},
            set(TARGETS),
        )

    def test_missing_target_is_rejected(self) -> None:
        candidate = candidate_manifest()
        candidate["artifacts"] = candidate["artifacts"][:1]
        identity = {key: value for key, value in candidate.items() if key != "candidate_id"}
        candidate["candidate_id"] = candidate_id(identity)

        with self.assertRaisesRegex(CandidateError, "exactly the four"):
            build_manifest(candidate)

    def test_cli_output_is_canonical(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            candidate_path = root / "candidate.json"
            output_path = root / "release-manifest.json"
            candidate_path.write_bytes(canonical_bytes(candidate_manifest()))

            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "build",
                    "--candidate-manifest",
                    str(candidate_path),
                    "--output",
                    str(output_path),
                ],
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(
                output_path.read_bytes(),
                canonical_bytes(json.loads(output_path.read_text(encoding="utf-8"))),
            )


if __name__ == "__main__":
    unittest.main()
