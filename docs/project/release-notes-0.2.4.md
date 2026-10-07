---
title: "Mitase 0.2.4 release notes"
description: "Default safety limits for authoring document size and semantic item count."
sidebar_position: 9
---

# Mitase 0.2.4 release notes

Mitase 0.2.4 adds default hard limits for `mitase/authoring/v2` documents.
The limits catch oversized source files and collections before they become
difficult to review as one bounded specification unit.

## Authoring document limits

Without an explicit configuration, each authoring document is limited to:

- 1,000 nonblank source lines. Blank lines do not count; comments do.
- 12 top-level Philosophy, Policy, Requirement, or Feature items. The count
  comes from the normalized semantic document.

Exceeding either limit produces an error: `MITASE-AUTHORING-005` for lines and
`MITASE-AUTHORING-006` for top-level items. These limits apply to the shared
workspace validation used by `mitase check`, `mitase validate workspace`, and
`mitase validate change`, including nested authoring directories.

This is a safety ceiling, not a recommended document size. Split unrelated
topics and oversized single items before reaching the ceiling. A repository that
already has a document over a default limit can now fail validation. Positive
repository-wide limits can be adjusted in `mitase.yaml`:

```yaml
validation:
  authoring:
    limits:
      max_nonblank_lines: 1600
      max_top_level_items: 20
```

Zero is not an unlimited value and is rejected. Mitase does not provide
per-path exceptions for these limits.
