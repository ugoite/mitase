# Ugoite current Operations acceptance corpus

This is a fixed, representative acceptance snapshot from `ugoite/ugoite`.
`corpus.yaml` pins the source revision and the selected source records. The
fixture translates one current requirement/feature slice into the v2 authoring
frontend while retaining the source IDs and exact target paths:

- `REQ-OPS-006#criterion.cli-surface`
- `FEAT-OPS-001#binding.implementation/target.config`
- `crates/ugoite-cli/src/config.rs::config_path`
- `crates/ugoite-cli/tests/test_cli_req_ops_006_config_helpers.rs::test_cli_req_ops_006_config_path_precedence_and_home_fallback`

The two Rust files are small materialized target excerpts. They preserve the
current Ugoite artifact identities needed to exercise Mitase resolution without
turning this fixture into a second checkout or changing Ugoite. The source
revision is evidence for the selected IDs and paths; the fixture is not a full
mirror of Ugoite's repository.

The acceptance test proves that the v2 documents normalize into one canonical
graph, the implementation and verification targets resolve exactly, and the
same graph is consumable through `mitase show` and `mitase query`.
