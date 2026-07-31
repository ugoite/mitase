# Troubleshooting

## `missing field \`kind\`` when validating a spec document

A v1 document must declare both `schema: syu/spec/v1` and a plural `kind` such as `philosophies`, `policies`, `requirements`, or `features`.

## `unknown adapter ...`

Enable the adapter in `syu.yaml` and use a supported adapter name.

## `changed implementation has no Criterion`

Every implementation binding must satisfy at least one requirement criterion.

## `required verification binding is missing`

Every accepted criterion in an executable slice needs its verification bindings present in the plan.

## `context pack exceeds serialized budget`

Reduce slice size or raise `work.slicing.max_total_bytes`.

Historical `linked_*`, `tests`, and `implementations` troubleshooting does not apply to the active v1 model.
