from __future__ import annotations

import re
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]
CANDIDATE = ROOT / ".github/workflows/release-candidate.yml"
PUBLISH = ROOT / ".github/workflows/release-publish.yml"
ACCEPTANCE = ROOT / "scripts/ci/check-release-acceptance.sh"
PACKAGE = ROOT / "scripts/ci/package-release.sh"
PACKAGE_SMOKE = ROOT / "scripts/ci/verify-packaged-release.sh"
RELEASE_MANIFEST = ROOT / "scripts/ci/release_manifest.py"
INSTALLER = ROOT / "scripts/install-mitase.sh"
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
        self.assertIn('DEFAULT_VERSION_SELECTOR="v0.1.3"', installer)
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
            "name: Run release-line acceptance gate",
            "bash scripts/ci/check-release-acceptance.sh",
            "scripts/ci/release_manifest.py build",
            "scripts/ci/release_manifest.py validate",
            "SHA256SUMS",
            "release-manifest.json",
        ):
            self.assertIn(required, workflow)
        self.assertNotIn("gh release create", workflow)
        self.assertNotIn("oras push", workflow)
        self.assertNotIn("__MITASE_RELEASE_TAG__", workflow)

    def test_candidate_checksum_artifacts_are_unique_per_target(self) -> None:
        workflow = CANDIDATE.read_text(encoding="utf-8")
        self.assertIn('checksum_file="checksums-${{ matrix.target }}.sha256"', workflow)
        self.assertNotIn("> checksums.sha256", workflow)

    def test_candidate_builds_four_versioned_unix_archives_and_smoke_tests_them(self) -> None:
        workflow = CANDIDATE.read_text(encoding="utf-8")
        for target in (
            "x86_64-unknown-linux-gnu",
            "aarch64-unknown-linux-gnu",
            "x86_64-apple-darwin",
            "aarch64-apple-darwin",
        ):
            self.assertIn(f"target: {target}", workflow)
        self.assertNotIn("x86_64-pc-windows-msvc", workflow)
        self.assertIn('"${{ needs.preflight.outputs.version }}"', workflow)
        self.assertIn("package-smoke:", workflow)
        self.assertIn("ubuntu-24.04-arm", workflow)
        self.assertIn("scripts/ci/verify-packaged-release.sh", workflow)
        self.assertIn("- package-smoke", workflow)
        self.assertNotIn(".zip", workflow)

    def test_package_tools_are_present(self) -> None:
        self.assertTrue(PACKAGE.is_file())
        self.assertTrue(PACKAGE_SMOKE.is_file())
        self.assertTrue(RELEASE_MANIFEST.is_file())
        self.assertTrue(PACKAGE.stat().st_mode & 0o111)
        self.assertTrue(PACKAGE_SMOKE.stat().st_mode & 0o111)
        self.assertTrue(RELEASE_MANIFEST.stat().st_mode & 0o111)
        self.assertNotIn("windows", PACKAGE.read_text(encoding="utf-8"))
        self.assertNotIn("zip", PACKAGE.read_text(encoding="utf-8"))

    def test_release_fallback_uses_the_selected_release_tag(self) -> None:
        installer = INSTALLER.read_text(encoding="utf-8")
        self.assertIn("tag = filtered[0][6]", installer)
        self.assertIn('archive_name = f\"mitase-{tag}-{target}.tar.gz\"', installer)
        self.assertNotIn("windows", installer)

    def test_promotion_downloads_by_run_id_and_never_builds(self) -> None:
        workflow = PUBLISH.read_text(encoding="utf-8")
        for required in (
            "candidate_run_id:",
            "candidate_id:",
            "run-id: ${{ inputs.candidate_run_id }}",
            "--candidate-id \"$INPUT_CANDIDATE_ID\"",
            "--artifact-root \"$artifact_root\"",
            "release_manifest.py validate",
            "sha256sum -c SHA256SUMS",
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

    def test_release_acceptance_gate_covers_both_release_lines(self) -> None:
        self.assertTrue(ACCEPTANCE.is_file())
        self.assertTrue(ACCEPTANCE.stat().st_mode & 0o111)
        script = ACCEPTANCE.read_text(encoding="utf-8")
        for required in (
            '0.1.*)',
            '0.2.*)',
            "current_release_policy_keeps_dual_source_during_0_1_x",
            "MITASE-SOURCE-001",
            "mitase migrate",
            "mitase_authoring_v2_preserves_the_pre_migration_canonical_graph",
            "self_hosted_config_preserves_the_exact_artifact_resolution_baseline",
            "check-architecture.py",
        ):
            self.assertIn(required, script)


if __name__ == "__main__":
    unittest.main()
