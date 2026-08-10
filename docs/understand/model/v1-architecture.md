# Mitase v1 architecture

Mitase stores one-way Philosophy → Policy → Requirement → Feature relations. Bindings own exact artifact targets; reverse relations are derived by `SpecIndex`. The CLI and Workbench projection share the workspace, planner, and validation libraries.
