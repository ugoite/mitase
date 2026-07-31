#!/usr/bin/env python3
"""Generate supplemental Docusaurus pages for the checked-in docs site."""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
OUTPUT_ROOT = REPO_ROOT / "docs" / "reference"


def write_page(path: Path, title: str, description: str, body: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(
        "\n".join(
            [
                "---",
                f'title: "{title}"',
                f'description: "{description}"',
                "---",
                "",
                body.strip(),
                "",
            ]
        ),
        encoding="utf-8",
    )


def main() -> None:
    write_page(
        OUTPUT_ROOT / "status" / "validation-report.md",
        "Repository validation report",
        "Checked-in validation report for the current repository state.",
        """# Repository validation report

This page anchors the repository's validation snapshot and gives the docs site a stable destination for the current report.

The checked-in report content is produced separately from the generated specification reference pages.
""",
    )


if __name__ == "__main__":
    main()
