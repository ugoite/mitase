# Reviewer workflow

For the active v1 surface, review starts with validation and exact work scope.

Suggested sequence:

```bash
mise run check:repo
mise run ci
```

What to check:

- changed artifacts still have explicit owners
- changed requirements and features still validate cleanly
- verification claims remain declarative and runners are not executed by
  Mitase
- execution context and delivery evidence are owned by repository tooling

Historical `trace`, `relate`, `log`, and `browse` commands are not part of the active root CLI.
