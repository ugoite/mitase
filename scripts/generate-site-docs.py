#!/usr/bin/env python3
"""Generate Docusaurus-friendly markdown pages from the configured spec root."""

from __future__ import annotations

import json
import shutil
from pathlib import Path

import yaml


REPO_ROOT = Path(__file__).resolve().parents[1]
DEFAULT_SPEC_ROOT = Path("docs/syu")
OUTPUT_ROOT = REPO_ROOT / "docs" / "reference" / "specification"

CATEGORY_METADATA = {
    "foundations": {
        "label": "Foundations and rules",
        "position": 1,
        "description": "The principles, policies, and requirements that make the repository's intent explicit.",
    },
    "capabilities": {
        "label": "Capabilities",
        "position": 2,
        "description": "The functional units that turn requirements into implemented repository behavior.",
    },
    "contracts": {
        "label": "Public contracts",
        "position": 3,
        "description": "The public entrypoints and typed contracts exposed by the self-hosted system.",
    },
    "workbench": {
        "label": "Workbench internals",
        "position": 4,
        "description": "The Workbench behaviors that support guided, bounded repository changes.",
    },
}


def display_name(value: str) -> str:
    return value.replace("_", " ").replace("-", " ").title()


def frontmatter(title: str, source_path: Path) -> list[str]:
    source_display = display_source_path(source_path)
    return [
        "---",
        f'title: "{title}"',
        f'description: "Generated reference for {source_display}"',
        "---",
        "",
    ]


def format_scalar(value: object, indent: int) -> list[str]:
    prefix = "  " * indent
    if value in ("", None):
        return [f"{prefix}- (empty)"]

    text = str(value).replace("<", "&lt;").replace(">", "&gt;")
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


def output_relative_path(relative_path: Path) -> Path:
    """Map source taxonomy to the reader-facing reference taxonomy."""

    parts = relative_path.parts
    if parts[0] in {"philosophies", "policies", "requirements"}:
        output_parts = ("foundations", *parts[1:])
    elif parts[0] == "features" and len(parts) > 1:
        section = {
            "capabilities": "capabilities",
            "public-entrypoints": "contracts",
            "workbench": "workbench",
        }.get(parts[1])
        output_parts = (section, *parts[2:]) if section else ("capabilities", *parts[1:])
    else:
        raise ValueError(f"unsupported specification path: {relative_path}")

    return Path(*output_parts).with_suffix(".md")


def resolve_spec_root() -> Path:
    config_path = REPO_ROOT / "syu.yaml"
    default_root = REPO_ROOT / DEFAULT_SPEC_ROOT
    if not config_path.is_file():
        return default_root

    document = yaml.safe_load(config_path.read_text(encoding="utf-8")) or {}
    if not isinstance(document, dict):
        return default_root

    workspace_config = document.get("workspace")
    if isinstance(workspace_config, dict):
        spec_roots = workspace_config.get("spec_roots")
        if isinstance(spec_roots, list):
            for raw_root in spec_roots:
                if isinstance(raw_root, str) and raw_root.strip():
                    candidate = Path(raw_root.strip())
                    if candidate.is_absolute():
                        return candidate
                    return REPO_ROOT / candidate

    spec_config = document.get("spec")
    if not isinstance(spec_config, dict):
        return default_root

    raw_root = spec_config.get("root")
    if not isinstance(raw_root, str) or not raw_root.strip():
        return default_root

    candidate = Path(raw_root.strip())
    if candidate.is_absolute():
        return candidate
    return REPO_ROOT / candidate


def relative_source_root(spec_root: Path) -> str:
    try:
        return spec_root.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return spec_root.as_posix()


def display_source_path(source_path: Path) -> str:
    try:
        return source_path.relative_to(REPO_ROOT).as_posix()
    except ValueError:
        return source_path.as_posix()


def write_markdown(source_path: Path, spec_root: Path) -> tuple[str, str]:
    relative_path = source_path.relative_to(spec_root)
    output_relative = output_relative_path(relative_path)
    output_path = OUTPUT_ROOT / output_relative
    output_path.parent.mkdir(parents=True, exist_ok=True)

    raw = source_path.read_text(encoding="utf-8")
    document = yaml.safe_load(raw) or {}
    if not isinstance(document, dict):
        document = {"content": document}

    title = page_title(relative_path, document)
    source_display = display_source_path(source_path)
    lines = frontmatter(title, source_path)
    lines.extend(
        [
            f"> Generated from `{source_display}`.",
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

    return title, output_relative.with_suffix("").as_posix()


def write_category_metadata(category: str, metadata: dict) -> None:
    category_path = OUTPUT_ROOT / category
    category_path.mkdir(parents=True, exist_ok=True)
    category_path.joinpath("_category_.json").write_text(
        json.dumps(
            {
                "label": metadata["label"],
                "position": metadata["position"],
                "collapsible": True,
                "collapsed": True,
                "link": {"type": "doc", "id": f"reference/specification/{category}/index"},
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def write_category_index(category: str, entries: list[tuple[str, str]]) -> None:
    metadata = CATEGORY_METADATA[category]
    lines = [
        "---",
        f'title: "{metadata["label"]}"',
        f'description: "{metadata["description"]}"',
        f'sidebar_position: {metadata["position"]}',
        "---",
        "",
        f"# {metadata['label']}",
        "",
        metadata["description"],
        "",
        "These pages are generated from the canonical YAML under `docs/syu/`.",
        "",
    ]
    for title, doc_link in entries:
        lines.append(f"- [{title}](./{Path(doc_link).name}.md)")
    lines.append("")
    (OUTPUT_ROOT / category / "index.md").write_text("\n".join(lines), encoding="utf-8")


def write_index(entries: list[tuple[str, str]], spec_root: Path) -> None:
    index_path = OUTPUT_ROOT / "index.md"
    source_root = relative_source_root(spec_root)
    lines = [
        "---",
        'title: "Specification Reference"',
        f'description: "Generated site pages for {source_root} YAML definitions."',
        "---",
        "",
        f"This section is generated from the YAML source under `{source_root}/`.",
        "",
        "## Browse by purpose",
        "",
    ]

    grouped: dict[str, list[tuple[str, str]]] = {category: [] for category in CATEGORY_METADATA}
    for title, doc_link in entries:
        grouped[Path(doc_link).parts[0]].append((title, doc_link))

    for category, metadata in CATEGORY_METADATA.items():
        lines.append(f"- [{metadata['label']}]({category}/index.md) — {metadata['description']}")

    lines.extend(["", "## Available documents", ""])
    for category in CATEGORY_METADATA:
        for title, doc_link in grouped[category]:
            lines.append(f"- [{title}]({doc_link}.md)")

    lines.append("")
    index_path.write_text("\n".join(lines), encoding="utf-8")

    for category, category_entries in grouped.items():
        write_category_metadata(category, CATEGORY_METADATA[category])
        write_category_index(category, category_entries)

    (OUTPUT_ROOT / "_category_.json").write_text(
        json.dumps(
            {
                "label": "Specification reference",
                "position": 1,
                "collapsible": True,
                "collapsed": True,
                "link": {"type": "doc", "id": "reference/specification/index"},
            },
            indent=2,
        )
        + "\n",
        encoding="utf-8",
    )


def main() -> None:
    spec_root = resolve_spec_root()
    if not spec_root.is_dir():
        raise SystemExit(f"configured spec root does not exist: {spec_root}")

    if OUTPUT_ROOT.exists():
        shutil.rmtree(OUTPUT_ROOT)
    OUTPUT_ROOT.mkdir(parents=True, exist_ok=True)

    entries = [write_markdown(path, spec_root) for path in sorted(spec_root.rglob("*.yaml"))]
    write_index(entries, spec_root)


if __name__ == "__main__":
    main()
