# Translation ledger

This ledger is part of the versioned acceptance fixture. The source revision
for every row is
`a872f4992bcb3633681eb0383e101453f00b32db` in `ugoite/ugoite`; source paths
are `docs/spec/philosophy/foundation.yaml` and
`docs/spec/policies/policies.yaml`.

## Translation rules

| Source shape | Mitase shape | Decision |
| --- | --- | --- |
| `product_design_principle` | Philosophy Principle `product-design-principle`, `applies_to: [product]` | Preserve the complete statement; the named source field becomes an explicit principle with product scope. |
| `coding_guideline` | Philosophy Principle `coding-guideline`, `applies_to: [code]` | Preserve the complete statement and keep implementation guidance distinct from the product principle. |
| Philosophy `linked_policies` | Policy Rule `governed_by` anchors | Author the relation once on the Policy Rule. The reverse Philosophy → Policy view is derived from `SpecIndex.rules_to_principles`; no reciprocal field is stored. |
| Policy `summary` and `description` | Policy `summary`, `description`, and one `governance` Rule | Preserve both texts. Mitase requires a Rule, so the source policy summary is also the rule statement. The source has no explicit RuleLevel; `should` is the conservative non-enforcing normalization. |
| Policy `linked_requirements` | Ledger only in this slice | These are Ugoite `REQCAT-*` downstream registry references. Requirements/Criteria migration is explicitly out of scope, so the graph does not invent partial downstream nodes. |
| Policy `linked_specifications` | Ledger only; future exact repository targets are Artifacts | These `SPEC-*` values identify Ugoite specification records/documents, not a new Mitase top-level kind. Their future exact document bindings belong in Artifact resolution. |

No row below requires a missing generic Mitase semantic. No Ugoite-specific
specification kind or compatibility parser is introduced.

## Source-item ledger

Every source item is listed. The `linked_philosophies` relation is preserved in
`spec/policies.yaml` and repeated in `corpus.yaml` so the test can verify the
authored-forward and derived-reverse forms.

| Source item | Non-clean source fields | Lost or deferred semantic information | Classification |
| --- | --- | --- | --- |
| `PHIL-001` | `product_design_principle`; `coding_guideline`; `linked_policies: [POL-001, POL-002, POL-004, POL-005, POL-007, POL-012, POL-014, POL-016]` | Field names become two typed Mitase Principles; the policy list is represented by authored Policy Rule governance and derived in reverse. No statement is lost. | representation difference |
| `PHIL-002` | `product_design_principle`; `coding_guideline`; `linked_policies: [POL-003, POL-004, POL-005, POL-011]` | Same two principle fields and derived reverse relation. No statement is lost. | representation difference |
| `PHIL-003` | `product_design_principle`; `coding_guideline`; `linked_policies: [POL-001, POL-002, POL-005, POL-006, POL-007, POL-008, POL-009, POL-012, POL-013, POL-014, POL-016]` | Same two principle fields and derived reverse relation. No statement is lost. | representation difference |
| `PHIL-004` | `product_design_principle`; `coding_guideline`; `linked_policies: [POL-006, POL-008, POL-009, POL-010, POL-011, POL-015]` | Same two principle fields and derived reverse relation. No statement is lost. | representation difference |
| `POL-001` | `linked_philosophies: [PHIL-001, PHIL-003]`; `linked_requirements: [REQCAT-STORAGE, REQCAT-API, REQCAT-INDEX]`; `linked_specifications: [SPEC-ARCH-OVERVIEW, SPEC-DM-OVERVIEW, SPEC-FEATURES-SQL, SPEC-DM-SQL-SESSIONS]` | Philosophy governance is preserved as Rule anchors. The downstream requirement links are not migrated. The specification IDs remain ledger evidence until exact source documents can be bound as Artifacts. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-002` | `linked_philosophies: [PHIL-001, PHIL-003]`; `linked_requirements: [REQCAT-API, REQCAT-ENTRY, REQCAT-INDEX, REQCAT-STORAGE]`; `linked_specifications: [SPEC-ARCH-OVERVIEW, SPEC-ARCH-DECISIONS, SPEC-API-REST]` | Governance is preserved. `REQCAT-*` is a downstream Ugoite registry layer and is deferred; `SPEC-*` remains an Artifact candidate, not a Mitase kind. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-003` | `linked_philosophies: [PHIL-002]`; `linked_requirements: [REQCAT-OPS, REQCAT-STORAGE, REQCAT-SECURITY]`; `linked_specifications: [SPEC-ARCH-STACK, SPEC-ARCH-FUTURE, SPEC-TESTING-CICD]` | Governance is preserved. Downstream registry relations are deferred and document references are not promoted to top-level specifications. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-004` | `linked_philosophies: [PHIL-001, PHIL-002]`; `linked_requirements: [REQCAT-API, REQCAT-INTEGRITY, REQCAT-SECURITY]`; `linked_specifications: [SPEC-ARCH-INTERFACE, SPEC-API-REST, SPEC-API-OPENAPI, SPEC-API-SURFACES]` | Governance is preserved. Requirement-category and specification-record links are retained as scoped evidence only. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-005` | `linked_philosophies: [PHIL-001, PHIL-002, PHIL-003]`; `linked_requirements: [REQCAT-OPS, REQCAT-E2E, REQCAT-INTEGRITY]`; `linked_specifications: [SPEC-ARCH-STACK, SPEC-API-SURFACES, SPEC-TESTING-STRATEGY, SPEC-TESTING-CICD]` | Governance is preserved. No partial Requirement or Criterion is invented; the specification records await exact Artifact bindings. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-006` | `linked_philosophies: [PHIL-003, PHIL-004]`; `linked_requirements: [REQCAT-FRONTEND, REQCAT-E2E, REQCAT-API]`; `linked_specifications: [SPEC-ARCH-INTERFACE, SPEC-UI-OVERVIEW, SPEC-UI-PAGES]` | Governance is preserved. Downstream requirement migration is deferred; UI and architecture records remain Artifact candidates. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-007` | `linked_philosophies: [PHIL-001, PHIL-003]`; `linked_requirements: [REQCAT-FORM, REQCAT-ENTRY, REQCAT-ASSET]`; `linked_specifications: [SPEC-DM-SCHEMA, SPEC-STORIES-CORE, SPEC-UI-PAGES]` | Governance is preserved. The source's downstream catalog layer is outside this fixture; referenced documents are not new spec kinds. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-008` | `linked_philosophies: [PHIL-003, PHIL-004]`; `linked_requirements: [REQCAT-API, REQCAT-OPS, REQCAT-E2E, REQCAT-INTEGRITY]`; `linked_specifications: [SPEC-TESTING-STRATEGY, SPEC-TESTING-CICD, SPEC-QUALITY-ERROR]` | Governance is preserved. Requirement/verification semantics are not partially reconstructed from Ugoite IDs; testing documents remain Artifact candidates. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-009` | `linked_philosophies: [PHIL-003, PHIL-004]`; `linked_requirements: [REQCAT-OPS, REQCAT-E2E, REQCAT-FRONTEND]`; `linked_specifications: [SPEC-TESTING-CICD, SPEC-TESTING-STRATEGY, SPEC-PRODUCT-METRICS]` | Governance is preserved. The source's requirement categories and document references are not silently treated as Mitase Requirements or Features. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-010` | `linked_philosophies: [PHIL-004]`; `linked_requirements: [REQCAT-SECURITY, REQCAT-OPS, REQCAT-API]`; `linked_specifications: [SPEC-SECURITY-OVERVIEW, SPEC-SECURITY-SANDBOX, SPEC-ARCH-FUTURE, SPEC-API-SURFACES]` | Governance is preserved. Security references are deferred as exact Artifacts and no Ugoite security kind is created. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-011` | `linked_philosophies: [PHIL-002, PHIL-004]`; `linked_requirements: [REQCAT-SECURITY, REQCAT-API, REQCAT-STORAGE]`; `linked_specifications: [SPEC-SECURITY-OVERVIEW, SPEC-SECURITY-SANDBOX, SPEC-API-MCP]` | Governance is preserved. The source's downstream registry relations are deferred; referenced security/MCP documents remain Artifact candidates. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-012` | `linked_philosophies: [PHIL-001, PHIL-003]`; `linked_requirements: [REQCAT-SEARCH, REQCAT-INDEX]`; `linked_specifications: [SPEC-FEATURES-REGISTRY, SPEC-STORIES-ADVANCED, SPEC-API-REST]` | Governance is preserved. Search and registry records are not promoted to new Mitase top-level kinds in a Philosophy/Policy-only fixture. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-013` | `linked_philosophies: [PHIL-003]`; `linked_requirements: [REQCAT-ENTRY, REQCAT-FRONTEND, REQCAT-API, REQCAT-OPS]`; `linked_specifications: [SPEC-STORIES-CORE, SPEC-STORIES-ADVANCED, SPEC-PRODUCT-METRICS, SPEC-STORIES-EXPERIMENTAL]` | Governance is preserved. Story and metrics records are deferred document/artifact references, not a new Mitase story kind. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-014` | `linked_philosophies: [PHIL-001, PHIL-003]`; `linked_requirements: [REQCAT-API, REQCAT-FRONTEND, REQCAT-STORAGE, REQCAT-ENTRY]`; `linked_specifications: [SPEC-ARCH-DECISIONS, SPEC-ARCH-FUTURE, SPEC-API-OPENAPI, SPEC-DM-DIRECTORY, SPEC-VERSIONS-KNOWLEDGE-COMPAT]` | Governance is preserved. The newer v0.1 compatibility document is contextual authority and an Artifact candidate; its presence does not require importing all downstream Ugoite semantics here. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-015` | `linked_philosophies: [PHIL-004]`; `linked_requirements: [REQCAT-API]`; `linked_specifications: [SPEC-API-MCP, SPEC-API-SURFACES]` | Governance is preserved. MCP documents remain exact Artifact candidates; the fixture does not create a Ugoite-specific MCP spec kind. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |
| `POL-016` | `linked_philosophies: [PHIL-001, PHIL-003]`; `linked_requirements: [REQCAT-OPS, REQCAT-STORAGE]`; `linked_specifications: [SPEC-VERSIONS-KNOWLEDGE-COMPAT, SPEC-ARCH-SPACE-CATALOG]` | Governance is preserved. The compatibility and catalog documents are contextual Artifacts; no generic Mitase extension is justified by their Ugoite-specific registry IDs. | representation difference; Ugoite legacy/governance drift; concept that should be an Artifact |

## Newer Foundation authority comparison

The following documents exist at the same pinned Ugoite revision but are newer
governance context than the machine-readable files' `last_updated` value. They
are deliberately not silently merged into the translated corpus.

| Context document | Observed difference | Fixture decision and classification |
| --- | --- | --- |
| `docs/architecture/principles/north-star.md` | The newer North Star makes operator-owned Space content, a single Catalog authority, append-only history, and the Knowledge/Work/Experience boundary explicit. The older four Philosophy entries express portability, topology, durable rules, and trusted AI, but do not enumerate those authority invariants. | Record the generation/layer drift; translate only the pinned machine-readable Philosophy source. The context document is a future exact Artifact, not a new Mitase kind. **Ugoite legacy/governance drift; concept that should be an Artifact.** |
| `docs/architecture/principles/knowledge-work-experience.md` | The newer document explicitly says Work, model context, and Experience runtime state may disappear and must not become a second Knowledge authority. The older Policy links mention AI, adapters, and compatibility without this precise failure/recovery boundary. | Keep the older Policy meaning and record the stronger newer boundary as contextual evidence. Do not create a Work or Experience specification kind. **Ugoite legacy/governance drift; concept that should be an Artifact.** |
| `docs/spec/versions/v0.1-knowledge-compatibility.md` | The newer compatibility floor freezes semantic ownership, authority, history, and adapter boundaries while leaving physical encodings and internal APIs replaceable. `POL-014` and `POL-016` point to this layer, but the source registry's flat `SPEC-*` links do not encode the distinction between semantic contract and physical implementation. | Retain the exact source links in the ledger, and treat the compatibility document as an Artifact candidate. This is governance-generation drift, not a missing generic Mitase semantic. **Ugoite legacy/governance drift; concept that should be an Artifact.** |

## Schema-extension decision

No generic Mitase schema extension is justified by this corpus. The observed
differences are source-field normalization, deferred downstream migration, or
Ugoite governance/document-generation drift. Source code, tests, OpenAPI,
architecture, UI, security, and CI material remain repository Artifacts under
the Mitase freeze; none becomes a top-level specification kind merely because
Ugoite's registry linked to it.
