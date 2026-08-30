# Troubleshooting

## `missing field \`kind\`` when validating a spec document

A v1 document must declare both `schema: mitase/spec/v1` and a plural `kind` such as `philosophies`, `policies`, `requirements`, or `features`.

## `unknown adapter ...`

Enable the adapter in `mitase.yaml` and use a supported adapter name.

## `changed implementation has no Criterion`

Every implementation binding must satisfy at least one requirement criterion.

## `required verification binding is missing`

Every implemented criterion needs an exact verification binding whose `covers`
list names the implementation target.

Historical `linked_*`, `tests`, and `implementations` troubleshooting does not apply to the active v1 model.
