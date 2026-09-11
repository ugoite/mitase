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


if __name__ == "__main__":
    unittest.main()
