#!/usr/bin/env python3
"""Generate supplemental Docusaurus pages for the checked-in docs site."""

from __future__ import annotations

from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[1]
OUTPUT_ROOT = REPO_ROOT / "docs" / "generated"


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
        OUTPUT_ROOT / "syu-report.md",
        "syu validation report",
        "Checked-in validation report for the current repository state.",
        """# syu validation report

This page anchors the repository's validation snapshot and gives the docs site a stable destination for the current report.

The checked-in report content is produced separately from the generated site-spec pages.
""",
    )

    write_page(
        OUTPUT_ROOT / "site-spec" / "features" / "documentation" / "docs.md",
        "Documentation / Docs",
        "Repository documentation feature reference.",
        """# Documentation / Docs

This page points readers at the documentation entry points that are part of the checked-in docs site.

- Guides live under `docs/guide`
- Generated specification reference lives under `docs/generated/site-spec`
- The validation report lives at `docs/generated/syu-report`
""",
    )


if __name__ == "__main__":
    main()
