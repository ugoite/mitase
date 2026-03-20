#!/usr/bin/env python3
"""Generate Docusaurus-friendly markdown pages from docs/spec YAML files."""

from __future__ import annotations

import shutil
from pathlib import Path

import yaml


REPO_ROOT = Path(__file__).resolve().parents[1]
SPEC_ROOT = REPO_ROOT / "docs" / "spec"
OUTPUT_ROOT = REPO_ROOT / "docs" / "generated" / "site-spec"


def display_name(value: str) -> str:
    return value.replace("_", " ").replace("-", " ").title()


def frontmatter(title: str, source_path: Path) -> list[str]:
    return [
        "---",
        f'title: "{title}"',
        f'description: "Generated reference for {source_path.as_posix()}"',
        "---",
        "",
    ]


def format_scalar(value: object, indent: int) -> list[str]:
    prefix = "  " * indent
    if value in ("", None):
        return [f"{prefix}- (empty)"]

    text = str(value)
    if "\n" not in text:
        return [f"{prefix}- {text}"]

    lines = [f"{prefix}- |"]
    lines.extend(f"{prefix}  {line}" if line else f"{prefix}  " for line in text.strip().splitlines())
    return lines


def render_value(value: object, indent: int = 0) -> list[str]:
    prefix = "  " * indent

    if isinstance(value, dict):
        if not value:
            return [f"{prefix}- (empty mapping)"]

        lines: list[str] = []
        for key, nested in value.items():
            if isinstance(nested, (dict, list)):
                lines.append(f"{prefix}- **{key}**:")
                lines.extend(render_value(nested, indent + 1))
            else:
                scalar_lines = format_scalar(nested, indent + 1)
                if len(scalar_lines) == 1 and not scalar_lines[0].strip().endswith("|"):
                    lines.append(f"{prefix}- **{key}**: {scalar_lines[0].strip()[2:]}")
                else:
                    lines.append(f"{prefix}- **{key}**:")
                    lines.extend(scalar_lines)
        return lines

    if isinstance(value, list):
        if not value:
            return [f"{prefix}- (empty list)"]

        lines: list[str] = []
        for item in value:
            if isinstance(item, (dict, list)):
                lines.append(f"{prefix}-")
                nested_lines = render_value(item, indent + 1)
                if nested_lines:
                    first, *rest = nested_lines
                    lines[-1] = f"{prefix}- {first.strip()[2:]}" if first.strip().startswith("- ") else lines[-1]
                    lines.extend(rest)
            else:
                lines.extend(format_scalar(item, indent))
        return lines

    return format_scalar(value, indent)


def page_title(relative_path: Path, document: dict) -> str:
    category = document.get("category")
    if category:
        return f"{category} / {display_name(relative_path.stem)}"
    return display_name(relative_path.stem)


def write_markdown(source_path: Path) -> tuple[str, str]:
    relative_path = source_path.relative_to(SPEC_ROOT)
    output_path = OUTPUT_ROOT / relative_path.with_suffix(".md")
    output_path.parent.mkdir(parents=True, exist_ok=True)

    raw = source_path.read_text(encoding="utf-8")
    document = yaml.safe_load(raw) or {}
    if not isinstance(document, dict):
        document = {"content": document}

    title = page_title(relative_path, document)
    lines = frontmatter(title, source_path.relative_to(REPO_ROOT))
    lines.extend(
        [
            f"> Generated from `{source_path.relative_to(REPO_ROOT).as_posix()}`.",
            "",
            "## Parsed content",
            "",
        ]
    )

    for key, value in document.items():
        lines.append(f"### {display_name(key)}")
        lines.append("")
        lines.extend(render_value(value))
        lines.append("")

    lines.extend(["## Source YAML", "", "```yaml", raw.rstrip(), "```", ""])
    output_path.write_text("\n".join(lines), encoding="utf-8")

    route_parts = list(relative_path.with_suffix("").parts)
    if len(route_parts) >= 2 and route_parts[-1] == route_parts[-2]:
        route_parts.pop()
    doc_link = "/docs/generated/site-spec"
    if route_parts:
        doc_link = f"{doc_link}/{'/'.join(route_parts)}"
    return title, doc_link


def write_index(entries: list[tuple[str, str]]) -> None:
    index_path = OUTPUT_ROOT / "index.md"
    lines = [
        "---",
        'title: "Specification Reference"',
        'description: "Generated site pages for docs/spec YAML definitions."',
        "---",
        "",
        "This section is generated from the YAML source under `docs/spec/`.",
        "",
        "## Available documents",
        "",
    ]

    for title, doc_link in entries:
        lines.append(f"- [{title}]({doc_link})")

    lines.append("")
    index_path.write_text("\n".join(lines), encoding="utf-8")


def main() -> None:
    if OUTPUT_ROOT.exists():
        shutil.rmtree(OUTPUT_ROOT)
    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)

    entries = [write_markdown(path) for path in sorted(SPEC_ROOT.rglob("*.yaml"))]
    write_index(entries)


if __name__ == "__main__":
    main()
