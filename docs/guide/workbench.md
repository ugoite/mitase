# Workbench

The active root CLI exposes one workbench projection command:

```bash
cargo run --quiet -- workbench project --workspace . --format json
```

That projection summarizes:

- workspace metadata
- validation state
- current spec graph counts
- optional work-request projection when a request file is supplied

The root CLI workbench surface is the projection command shown above.
