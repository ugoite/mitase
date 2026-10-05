# Troubleshooting

## `missing field \`kind\`` when validating a spec document

Legacy v1 documents are not workspace input. Migrate one explicitly and keep
the emitted v0.2 source under the configured `spec_roots`:

```bash
mitase migrate docs/mitase/legacy.yaml --stdout > docs/mitase/migrated.yaml
```

A v2 document must declare `schema: mitase/authoring/v2` and a `kind` such
as `philosophies`, `policies`, `requirements`, `features`, or the short
`requirement` contract.

## `unknown adapter ...`

Enable the adapter in `mitase.yaml` and use a supported adapter name.

## `changed implementation has no Criterion`

Every implementation binding must satisfy at least one requirement criterion.

## `required verification binding is missing`

Every implemented criterion needs an exact verification binding whose `covers`
list names the implementation target.

Historical `linked_*`, `tests`, and `implementations` troubleshooting does not apply to the active v1 model.
