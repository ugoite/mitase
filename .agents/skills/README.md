# Repository skills

These skills are the repository-level delivery contract for Mitase. They keep
implementation, review, validation, CI triage, PR safety, and release work
separate from the Mitase product boundary.

Available skills:

- [`mitase-orient/SKILL.md`](mitase-orient/SKILL.md): read the frozen boundary
  and route a change to the right repository surface.
- [`mitase-implement/SKILL.md`](mitase-implement/SKILL.md): implement a focused
  Issue and drive it through review convergence.
- [`mitase-validate/SKILL.md`](mitase-validate/SKILL.md): choose and interpret
  root Mise validation tasks and hosted lanes.
- [`mitase-ci-triage/SKILL.md`](mitase-ci-triage/SKILL.md): classify and repair
  the first actionable CI failure.
- [`codex-pr-safety/SKILL.md`](codex-pr-safety/SKILL.md): create public-safe
  Issue and pull request content.
- [`mitase-release/SKILL.md`](mitase-release/SKILL.md): keep release work
  immutable and separate from normal CI.

The existing `codex-github-workflows/` directory remains available for the
generic GitHub workflow helpers.

## Install

Copy the `codex-github-workflows` directory into your Codex skills directory.

## Optional standalone prompt generation

The skill includes a helper script for copy-paste prompts:

```bash
python skills/codex-github-workflows/scripts/build_prompt.py pr_merge 123
python skills/codex-github-workflows/scripts/build_prompt.py pr_review 123
python skills/codex-github-workflows/scripts/build_prompt.py implementation 45
python skills/codex-github-workflows/scripts/build_prompt.py issue_creation 3
```
