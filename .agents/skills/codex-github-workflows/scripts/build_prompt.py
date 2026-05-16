#!/usr/bin/env python3
"""Build standalone Codex prompts for GitHub workflows.

This helper is optional. The skill itself should usually be enough when installed
in Codex, but this script is useful when a user wants a copy-paste prompt for a
separate Codex session.
"""

from __future__ import annotations

import argparse
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / "references" / "workflows"

COMMON = """Role: You are a senior autonomous coding agent working in this GitHub repository.

Working style:
- Outcome first: finish the requested GitHub task safely with the fewest useful tool loops.
- Do not narrate routine progress or print long command output/diffs unless needed to explain a blocker.
- Use an initial visible update only when the task is ambiguous, risky, or requires multiple write actions.

Repository safety:
- Start with `git status --short`.
- Do not overwrite staged or user changes.
- Avoid destructive cleanup unless the unrelated local change is clearly safe.
- Before any GitHub write action, confirm eligibility and blockers from current data.

Evidence budget:
- Use one metadata call, one CI/check call, then only the smallest additional query needed for a missing fact.
- Read changed files and direct behavior definitions only; do not perform broad audits.
- Do not repeat a failing command unless command shape, inputs, or strategy changed.
- Prefer known-good command forms. Avoid unsupported `gh pr diff --stat`, path args after `gh pr diff`, `gh pr merge --yes`, and `--delete-branch` when merge queue is enabled.
- When shell patterns contain backticks, quote safely with single quotes or avoid the backticks.

Validation budget:
- Run validation only when it can change the decision or prove a fix.
- Use the narrowest relevant test/check; widen once at most when risk requires it.
- If validation cannot run, report why and the next-best check.

GitHub writing rules:
- For PR bodies or long comments, write Markdown to a temp file and pass `--body-file` or equivalent.
- Stage only intended files. If hooks modify files, fix/stage/commit and retry once.
- Do not install local toolchains just to satisfy pre-push hooks. If one validated push fails only because local environment prerequisites are missing, push once with `--no-verify` and rely on remote CI.

CI/queue budget:
- Check CI once after PR creation, push, or merge request.
- If pending, poll at most twice. If the PR is in a merge queue, verify queue entry once and stop queued/pending.
""".strip()

FILES = {
    "pr_merge": "pr-merge.md",
    "pr_review": "pr-review.md",
    "implementation": "implementation.md",
    "issue_creation": "issue-creation.md",
}


def load_workflow(key: str) -> str:
    text = (WORKFLOWS / FILES[key]).read_text(encoding="utf-8")
    lines = []
    for line in text.splitlines():
        if line.startswith("# "):
            continue
        lines.append(line)
    return "\n".join(lines).strip()


def render(key: str, number: str) -> str:
    number = number.lstrip("#")
    workflow = load_workflow(key)
    if key in {"pr_merge", "pr_review"}:
        workflow = workflow.replace("<PR>", number).replace("#<PR>", f"#{number}")
    elif key == "implementation":
        workflow = workflow.replace("<ISSUE>", number).replace("#<ISSUE>", f"#{number}")
    elif key == "issue_creation":
        workflow = workflow.replace("the requested maximum number", number)
    return f"{COMMON}\n\n{workflow}\n"


def main() -> int:
    parser = argparse.ArgumentParser(description="Build a standalone Codex prompt.")
    parser.add_argument(
        "workflow",
        choices=sorted(FILES),
        help="Workflow key: pr_merge, pr_review, implementation, or issue_creation.",
    )
    parser.add_argument(
        "number",
        help="PR number, issue number, or max issue count for issue_creation.",
    )
    args = parser.parse_args()
    print(render(args.workflow, args.number))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
