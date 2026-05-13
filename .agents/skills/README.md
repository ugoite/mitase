# Codex GitHub Workflow Skills

This archive contains one Codex/OpenAI Agent Skill:

- `codex-github-workflows/` — outcome-first GitHub repository workflows for PR merge/queue, PR review, issue implementation, and issue creation.

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
