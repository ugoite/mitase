from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]
CANDIDATE = ROOT / ".github/workflows/release-candidate.yml"
PUBLISH = ROOT / ".github/workflows/release-publish.yml"
PINNED_ACTION = re.compile(r"^\s*(?:-\s*)?uses:\s+[^\s@]+@[0-9a-f]{40}(?:\s+#.*)?$")


class ReleaseWorkflowTests(unittest.TestCase):
    def test_release_delivery_is_split_into_candidate_and_promotion(self) -> None:
        self.assertFalse((ROOT / ".github/workflows/release-artifacts.yml").exists())
        self.assertTrue(CANDIDATE.is_file())
        self.assertTrue(PUBLISH.is_file())

    def test_old_release_authorities_are_removed(self) -> None:
        self.assertFalse((ROOT / ".github/workflows/release-please.yml").exists())
        self.assertFalse((ROOT / ".release-please-manifest.json").exists())
        self.assertFalse((ROOT / "release-please-config.json").exists())
        installer = (ROOT / "scripts/install-mitase.sh").read_text(encoding="utf-8")
        self.assertIn('DEFAULT_VERSION_SELECTOR="v0.1.1"', installer)
        self.assertNotIn("__MITASE_RELEASE_TAG__", installer)
        candidate = CANDIDATE.read_text(encoding="utf-8")
        self.assertIn('Path("Cargo.toml")', candidate)
        self.assertIn(r"\[workspace\.package\]", candidate)

    def test_candidate_requires_source_sha_and_builds_manifest_after_packaging(self) -> None:
        workflow = CANDIDATE.read_text(encoding="utf-8")
        for required in (
            "workflow_dispatch:",
            "source_sha:",
            "ref: ${{ inputs.source_sha }}",
            "ref: ${{ needs.preflight.outputs.source_sha }}",
            "rustup component add rustfmt clippy",
            "release_candidate.py build",
            "release_candidate.py validate",
            "name: mitase-release-candidate",
        ):
            self.assertIn(required, workflow)
        self.assertNotIn("gh release create", workflow)
        self.assertNotIn("oras push", workflow)
        self.assertNotIn("__MITASE_RELEASE_TAG__", workflow)

    def test_candidate_checksum_artifacts_are_unique_per_target(self) -> None:
        workflow = CANDIDATE.read_text(encoding="utf-8")
        self.assertIn('checksum_file="checksums-${{ matrix.target }}.sha256"', workflow)
        self.assertNotIn("> checksums.sha256", workflow)

    def test_promotion_downloads_by_run_id_and_never_builds(self) -> None:
        workflow = PUBLISH.read_text(encoding="utf-8")
        for required in (
            "candidate_run_id:",
            "candidate_id:",
            "run-id: ${{ inputs.candidate_run_id }}",
            "--candidate-id \"$INPUT_CANDIDATE_ID\"",
            "--artifact-root \"$artifact_root\"",
            "gh release create",
            "publish-package.sh",
        ):
            self.assertIn(required, workflow)
        for forbidden in ("cargo build", "cargo install", "docker build", "release-artifacts.yml"):
            self.assertNotIn(forbidden, workflow)
        self.assertLess(
            workflow.index("name: Verify candidate identity and exact artifact bytes"),
            workflow.index("name: Publish exact release assets"),
        )
        self.assertLess(
            workflow.index("name: Publish exact release assets"),
            workflow.index("name: Publish exact package archives"),
        )

    def test_candidate_manifest_checksum_is_relative_to_bundle_root(self) -> None:
        workflow = CANDIDATE.read_text(encoding="utf-8")
        self.assertIn(
            "sha256sum candidate-manifest.json > candidate-manifest.json.sha256",
            workflow,
        )
        self.assertNotIn(
            "sha256sum target/candidate-bundle/candidate-manifest.json",
            workflow,
        )

    def test_new_workflow_actions_are_commit_pinned(self) -> None:
        for workflow_path in (CANDIDATE, PUBLISH):
            for line in workflow_path.read_text(encoding="utf-8").splitlines():
                if "uses:" in line and "./" not in line:
                    self.assertRegex(line, PINNED_ACTION)


if __name__ == "__main__":
    unittest.main()
