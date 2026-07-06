# Existing repository

Add `syu` to an existing repository by introducing one workspace config and one v1 spec tree.

Start with a minimal `syu.yaml`:

```yaml
# x-release-please-start-version
version: 0.0.1-alpha.8
# x-release-please-end
schema: syu/config/v1
workspace:
  spec_roots: [docs/syu]
  artifact_roots: [src, tests]
  excludes: []
profiles: { active: [], custom: {} }
validation:
  preset: standard
  deny_warnings: false
  rules: {}
  changed:
    require_owned_changes: false
work:
  slicing:
    max_editable_files: 4
    max_editable_symbols: 8
    max_verification_targets: 4
    max_readonly_targets: 8
    max_total_bytes: 16384
  context:
    include_parent_principles: false
    include_parent_rules: false
adapters: { enabled: [rust, typescript, shell, python, go, markdown, yaml, json] }
```

Then add four v1 documents under `docs/syu`:

- one philosophies document
- one policies document
- one requirements document
- one features document

After each step, run:

```bash
cargo run --quiet -- validate .
```

Examples are the reference bootstrap path for the current CLI.
