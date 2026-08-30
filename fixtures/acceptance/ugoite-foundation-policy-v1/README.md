# Ugoite Philosophy/Policy acceptance corpus

This is a versioned Mitase acceptance fixture for the Ugoite governance corpus.
The source is pinned in `corpus.yaml` to one commit of `ugoite/ugoite`; the
translated graph is in `spec/`.

The fixture represents all four source Philosophies and all sixteen source
Policies. Ugoite's authored `linked_philosophies` relation is represented by
Mitase Policy Rule `governed_by` anchors. The reverse Philosophy → Policy view
is derived from the canonical index; reciprocal `linked_policies` fields are
not copied into the Mitase documents.

`translation-ledger.md` records every source item, the fields that do not map
cleanly in this focused Philosophy/Policy slice, the information that is not
carried into the graph, and its classification. In particular, it records the
newer Foundation authority documents as governance context rather than
silently treating their additional concepts as a schema gap.

The fixture deliberately does not replace Ugoite's live registry, add Ugoite
specific specification kinds, migrate Requirements/Criteria/Features, or wire
Mitase into Ugoite CI. Those are separate migration decisions.
