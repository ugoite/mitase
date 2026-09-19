from __future__ import annotations

import subprocess
import tempfile
import unittest
from pathlib import Path


ROOT = Path(__file__).parents[2]
SCRIPT = ROOT / "scripts/ci/package-release.sh"


class PackageReleaseTests(unittest.TestCase):
    def test_unix_archive_is_versioned_and_contains_only_mitase(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "mitase"
            output = root / "artifacts"
            binary.write_text(
                "#!/usr/bin/env sh\nprintf 'mitase 0.1.8\\n'\n", encoding="utf-8"
            )
            binary.chmod(0o755)

            result = subprocess.run(
                [
                    str(SCRIPT),
                    "x86_64-unknown-linux-gnu",
                    str(binary),
                    str(output),
                    "v0.1.8",
                ],
                capture_output=True,
                text=True,
            )

            self.assertEqual(result.returncode, 0, result.stderr)
            archive = output / "mitase-v0.1.8-x86_64-unknown-linux-gnu.tar.gz"
            self.assertTrue(archive.is_file())
            self.assertTrue((output / f"{archive.name}.sha256").is_file())
            entries = subprocess.check_output(
                ["tar", "-tzf", str(archive)], text=True
            ).splitlines()
            self.assertEqual(entries, ["mitase"])
            smoke = subprocess.run(
                [
                    str(ROOT / "scripts/ci/verify-packaged-release.sh"),
                    "v0.1.8",
                    "x86_64-unknown-linux-gnu",
                    str(archive),
                    str(root),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(smoke.returncode, 0, smoke.stderr)

    def test_v2_package_smoke_checks_v1_rejection_and_read_only_migration(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            binary = root / "mitase"
            output = root / "artifacts"
            binary.write_text(
                """#!/usr/bin/env python3
import json
import sys

if sys.argv[1:] == ["--version"]:
    print("mitase 0.2.0")
    raise SystemExit(0)
if sys.argv[1] == "migrate":
    print("schema: mitase/authoring/v2")
    raise SystemExit(0)
if sys.argv[1] in {"check", "validate"}:
    if "valid-web-app" in " ".join(sys.argv):
        print(json.dumps({
            "diagnostics": [{
                "code": "MITASE-SOURCE-001",
                "suggested_action": "run mitase migrate <source> --stdout",
                "primary": {"path": "spec/feature.yaml"},
            }]
        }))
        raise SystemExit(1)
    raise SystemExit(0)
raise SystemExit(2)
""",
                encoding="utf-8",
            )
            binary.chmod(0o755)

            result = subprocess.run(
                [
                    str(SCRIPT),
                    "x86_64-unknown-linux-gnu",
                    str(binary),
                    str(output),
                    "v0.2.0",
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.returncode, 0, result.stderr)

            archive = output / "mitase-v0.2.0-x86_64-unknown-linux-gnu.tar.gz"
            smoke = subprocess.run(
                [
                    str(ROOT / "scripts/ci/verify-packaged-release.sh"),
                    "v0.2.0",
                    "x86_64-unknown-linux-gnu",
                    str(archive),
                    str(root),
                ],
                capture_output=True,
                text=True,
            )
            self.assertEqual(smoke.returncode, 0, smoke.stderr)


if __name__ == "__main__":
    unittest.main()
