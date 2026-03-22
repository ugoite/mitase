// FEAT-APP-001

// FEAT-APP-001

import { useEffect, useMemo, useState } from "react";
import type {
  AppPayload,
  BrowserDocument,
  BrowserWorkspace,
  BrowserTraceGroup,
  ReferencedRule,
  SectionKind,
  ValidationIssue,
} from "./types";

type WasmModule = {
  default: () => Promise<void>;
  build_browser_workspace_from_js: (payload: AppPayload) => BrowserWorkspace;
};

const SECTION_ORDER: SectionKind[] = ["philosophy", "policies", "features", "requirements"];

const SECTION_COPY: Record<SectionKind, string> = {
  philosophy: "Stable project intent and enduring values.",
  policies: "Repository-wide rules that operationalize philosophy.",
  features: "Implemented capabilities that satisfy requirements.",
  requirements: "Specific obligations with traceable delivery evidence.",
};

function App() {
  const [workspace, setWorkspace] = useState<BrowserWorkspace | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedSection, setSelectedSection] = useState<SectionKind>("philosophy");
  const [selectedDocumentPath, setSelectedDocumentPath] = useState("");
  const [selectedItemId, setSelectedItemId] = useState("");
  const [selectedIssueCode, setSelectedIssueCode] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;

    const loadWorkspace = async () => {
      try {
        const [wasmModule, response] = await Promise.all([
          import("./wasm/syu_app_wasm.js") as Promise<WasmModule>,
          fetch("/api/app-data.json"),
        ]);

        if (!response.ok) {
          throw new Error(`Failed to load app data: ${response.status} ${response.statusText}`);
        }

        const payload = (await response.json()) as AppPayload;
        await wasmModule.default();
        const browserWorkspace = wasmModule.build_browser_workspace_from_js(payload);

        if (cancelled) {
          return;
        }

        setWorkspace(browserWorkspace);
        const nextSection = firstPopulatedSection(browserWorkspace) ?? "philosophy";
        setSelectedSection(nextSection);
        const firstDocument = browserWorkspace.sections.find(
          (section) => section.kind === nextSection,
        )?.documents[0];
        setSelectedDocumentPath(firstDocument?.path ?? "");
        setSelectedItemId(firstDocument?.items[0]?.id ?? "");
        setSelectedIssueCode(browserWorkspace.validation.issues[0]?.code ?? null);
      } catch (loadError) {
        if (!cancelled) {
          setError(loadError instanceof Error ? loadError.message : "Failed to load syu app");
        }
      } finally {
        if (!cancelled) {
          setLoading(false);
        }
      }
    };

    void loadWorkspace();

    return () => {
      cancelled = true;
    };
  }, []);

  const currentSection = useMemo(() => {
    return workspace?.sections.find((section) => section.kind === selectedSection) ?? null;
  }, [selectedSection, workspace]);

  const currentDocument = useMemo(() => {
    if (!currentSection) {
      return null;
    }

    return (
      currentSection.documents.find((document) => document.path === selectedDocumentPath) ??
      currentSection.documents[0] ??
      null
    );
  }, [currentSection, selectedDocumentPath]);

  const currentItem = useMemo(() => {
    if (!currentDocument) {
      return null;
    }

    return (
      currentDocument.items.find((item) => item.id === selectedItemId) ??
      currentDocument.items[0] ??
      null
    );
  }, [currentDocument, selectedItemId]);

  const documentGroups = useMemo(() => {
    if (!currentSection) {
      return [] as Array<[string, BrowserDocument[]]>;
    }

    const grouped = new Map<string, BrowserDocument[]>();
    for (const document of currentSection.documents) {
      const key = document.folder_segments.join(" / ") || "workspace root";
      const docs = grouped.get(key) ?? [];
      docs.push(document);
      grouped.set(key, docs);
    }

    return [...grouped.entries()];
  }, [currentSection]);

  const activeIssue = useMemo(() => {
    return (
      workspace?.validation.issues.find((issue) => issue.code === selectedIssueCode) ??
      workspace?.validation.issues[0] ??
      null
    );
  }, [selectedIssueCode, workspace]);

  const activeRule = useMemo(() => {
    if (!workspace || !activeIssue) {
      return null;
    }

    return (
      workspace.validation.referenced_rules.find((rule) => rule.code === activeIssue.code) ?? null
    );
  }, [activeIssue, workspace]);

  const selectSection = (nextSection: SectionKind) => {
    if (!workspace) {
      return;
    }

    const section = workspace.sections.find((candidate) => candidate.kind === nextSection);
    setSelectedSection(nextSection);
    setSelectedDocumentPath(section?.documents[0]?.path ?? "");
    setSelectedItemId(section?.documents[0]?.items[0]?.id ?? "");
  };

  const selectDocument = (document: BrowserDocument) => {
    setSelectedDocumentPath(document.path);
    setSelectedItemId(document.items[0]?.id ?? "");
  };

  const jumpToItem = (id: string) => {
    if (!workspace) {
      return;
    }

    const target = workspace.item_index.get(id);
    if (!target) {
      return;
    }

    setSelectedSection(target.kind);
    setSelectedDocumentPath(target.document_path);
    setSelectedItemId(id);
  };

  if (loading) {
    return (
      <div className="app-shell flex items-center justify-center px-6 text-slate-300">
        <div className="app-glass rounded-3xl border border-sky-400/20 px-6 py-5 shadow-2xl shadow-sky-950/30">
          Loading syu app...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="app-shell flex items-center justify-center px-6 text-slate-100">
        <div className="app-glass max-w-2xl rounded-3xl border border-rose-500/40 px-8 py-6 shadow-2xl shadow-rose-950/30">
          <p className="text-sm uppercase tracking-[0.3em] text-rose-300">syu app</p>
          <h1 className="mt-3 text-2xl font-semibold">The browser workspace could not load.</h1>
          <p className="mt-3 text-sm leading-7 text-slate-300">{error}</p>
        </div>
      </div>
    );
  }

  if (!workspace) {
    return null;
  }

  return (
    <div className="app-shell text-slate-100">
      <header className="sticky top-0 z-30 border-b border-white/10 bg-slate-950/85 backdrop-blur-2xl">
        <div className="mx-auto flex max-w-7xl flex-col gap-6 px-4 py-5 sm:px-6 lg:px-8">
          <div className="flex flex-col gap-5 lg:flex-row lg:items-end lg:justify-between">
            <div className="max-w-3xl">
              <p className="text-xs font-semibold uppercase tracking-[0.35em] text-sky-300">
                FEAT-APP-001
              </p>
              <h1 className="mt-2 text-3xl font-semibold tracking-tight text-white sm:text-4xl">
                syu app
              </h1>
              <p className="mt-3 text-sm leading-7 text-slate-300 sm:text-base">
                Explore philosophy, policies, features, requirements, and the current validation
                state with a local browser UI powered by Rust and WebAssembly.
              </p>
            </div>
            <dl className="grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
              <SummaryCard label="workspace" value={workspace.workspace_root} />
              <SummaryCard label="spec root" value={workspace.spec_root} />
              <SummaryCard
                label="issues"
                value={`${workspace.validation.issues.length} ${workspace.validation.issues.length === 1 ? "issue" : "issues"}`}
              />
              <SummaryCard
                label="trace coverage"
                value={`${workspace.validation.trace_summary.requirement_traces.validated}/${workspace.validation.trace_summary.requirement_traces.declared} req · ${workspace.validation.trace_summary.feature_traces.validated}/${workspace.validation.trace_summary.feature_traces.declared} feat`}
              />
            </dl>
          </div>

          <nav aria-label="Top level sections" className="flex flex-wrap gap-2">
            {SECTION_ORDER.map((section) => {
              const entry = workspace.sections.find((candidate) => candidate.kind === section);
              const isActive = section === selectedSection;
              const count = entry?.documents.length ?? 0;
              return (
                <button
                  key={section}
                  type="button"
                  onClick={() => selectSection(section)}
                  className={`rounded-full border px-4 py-2 text-sm font-medium transition ${
                    isActive
                      ? "border-sky-400 bg-sky-400/20 text-sky-100 shadow-lg shadow-sky-950/40"
                      : "border-white/10 bg-white/5 text-slate-300 hover:border-sky-400/40 hover:text-white"
                  }`}
                >
                  {section} <span className="ml-2 text-xs text-slate-400">{count} files</span>
                </button>
              );
            })}
          </nav>

          <div className="flex flex-col gap-3">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">submenu</p>
                <p className="mt-1 text-sm text-slate-300">{SECTION_COPY[selectedSection]}</p>
              </div>
              <p className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
                {currentSection?.documents.length ?? 0} documents
              </p>
            </div>
            <div className="flex flex-col gap-3 lg:flex-row lg:flex-wrap">
              {documentGroups.length === 0 ? (
                <div className="rounded-2xl border border-dashed border-white/10 px-4 py-3 text-sm text-slate-400">
                  No {selectedSection} documents were discovered in this workspace.
                </div>
              ) : (
                documentGroups.map(([group, documents]) => (
                  <div
                    key={group}
                    className="app-glass min-w-0 rounded-2xl border border-white/10 px-3 py-3"
                  >
                    <p className="px-1 text-[11px] font-semibold uppercase tracking-[0.25em] text-slate-500">
                      {group}
                    </p>
                    <div className="mt-2 flex flex-wrap gap-2">
                      {documents.map((document) => {
                        const isActive = currentDocument?.path === document.path;
                        return (
                          <button
                            key={document.path}
                            type="button"
                            onClick={() => selectDocument(document)}
                            className={`rounded-xl border px-3 py-2 text-left text-sm transition ${
                              isActive
                                ? "border-sky-400/70 bg-sky-400/15 text-sky-100"
                                : "border-white/10 bg-slate-900/60 text-slate-300 hover:border-sky-400/30 hover:text-white"
                            }`}
                          >
                            <span className="block font-medium">{document.title}</span>
                            <span className="mt-1 block text-xs text-slate-500">
                              {document.path}
                            </span>
                          </button>
                        );
                      })}
                    </div>
                  </div>
                ))
              )}
            </div>
          </div>
        </div>
      </header>

      <main className="mx-auto grid max-w-7xl gap-6 px-4 py-6 sm:px-6 lg:grid-cols-12 lg:px-8">
        <section className="space-y-6 lg:col-span-8">
          <section className="grid gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <CountCard
              label="philosophy"
              value={workspace.validation.definition_counts.philosophies}
            />
            <CountCard label="policies" value={workspace.validation.definition_counts.policies} />
            <CountCard label="features" value={workspace.validation.definition_counts.features} />
            <CountCard
              label="requirements"
              value={workspace.validation.definition_counts.requirements}
            />
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6">
            <div className="flex flex-col gap-4 lg:flex-row lg:items-start lg:justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">document</p>
                <h2 className="mt-2 text-2xl font-semibold text-white">
                  {currentDocument?.title ?? "No document selected"}
                </h2>
                {currentDocument ? (
                  <p className="mt-2 text-sm text-slate-400">{currentDocument.path}</p>
                ) : null}
              </div>
              {currentDocument?.parse_error ? (
                <div className="rounded-2xl border border-amber-400/30 bg-amber-400/10 px-4 py-3 text-sm text-amber-100">
                  <p className="font-medium">
                    This document could not be parsed into the expected layer model.
                  </p>
                  <p className="mt-2 text-xs leading-6 text-amber-50/80">
                    {currentDocument.parse_error}
                  </p>
                </div>
              ) : null}
            </div>

            {currentDocument && currentDocument.items.length > 1 ? (
              <div className="mt-5 flex flex-wrap gap-2 border-t border-white/10 pt-5">
                {currentDocument.items.map((item) => {
                  const isActive = item.id === currentItem?.id;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setSelectedItemId(item.id)}
                      className={`rounded-full border px-3 py-2 text-sm transition ${
                        isActive
                          ? "border-sky-400 bg-sky-400/15 text-sky-100"
                          : "border-white/10 bg-white/5 text-slate-300 hover:border-sky-400/40 hover:text-white"
                      }`}
                    >
                      {item.id}
                    </button>
                  );
                })}
              </div>
            ) : null}

            {currentItem ? (
              <article className="mt-6 space-y-6">
                <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
                  <div>
                    <p className="text-xs uppercase tracking-[0.3em] text-slate-500">
                      selected item
                    </p>
                    <h3 className="mt-2 text-2xl font-semibold text-white">
                      {currentItem.id} — {currentItem.title}
                    </h3>
                  </div>
                  <div className="flex flex-wrap gap-2">
                    {currentItem.status ? (
                      <MetaPill label="status" value={currentItem.status} />
                    ) : null}
                    {currentItem.priority ? (
                      <MetaPill label="priority" value={currentItem.priority} />
                    ) : null}
                    <MetaPill label="layer" value={currentItem.kind} />
                  </div>
                </div>

                <div className="grid gap-4 xl:grid-cols-2">
                  <InfoPanel
                    title="Summary"
                    content={currentItem.summary}
                    emptyCopy="No summary is authored for this item."
                  />
                  <InfoPanel
                    title="Description"
                    content={currentItem.description}
                    emptyCopy="No description is authored for this item."
                  />
                  <InfoPanel
                    title="Product design principle"
                    content={currentItem.product_design_principle}
                    emptyCopy="This layer does not use product design principles."
                  />
                  <InfoPanel
                    title="Coding guideline"
                    content={currentItem.coding_guideline}
                    emptyCopy="This layer does not use coding guidelines."
                  />
                </div>

                <div className="grid gap-4 xl:grid-cols-2">
                  <RelationshipPanel
                    label="Linked philosophies"
                    ids={currentItem.linked_philosophies}
                    jumpToItem={jumpToItem}
                  />
                  <RelationshipPanel
                    label="Linked policies"
                    ids={currentItem.linked_policies}
                    jumpToItem={jumpToItem}
                  />
                  <RelationshipPanel
                    label="Linked requirements"
                    ids={currentItem.linked_requirements}
                    jumpToItem={jumpToItem}
                  />
                  <RelationshipPanel
                    label="Linked features"
                    ids={currentItem.linked_features}
                    jumpToItem={jumpToItem}
                  />
                </div>

                <TracePanel label="Tests" groups={currentItem.tests} />
                <TracePanel label="Implementations" groups={currentItem.implementations} />
              </article>
            ) : currentDocument ? (
              <div className="mt-6 rounded-2xl border border-dashed border-white/10 px-4 py-6 text-sm text-slate-400">
                This document is available as raw YAML, but it does not expose any parsed items for
                this layer.
              </div>
            ) : (
              <div className="mt-6 rounded-2xl border border-dashed border-white/10 px-4 py-6 text-sm text-slate-400">
                Choose a document from the submenu to inspect its content.
              </div>
            )}
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">raw document</p>
                <h2 className="mt-2 text-xl font-semibold text-white">Checked-in YAML</h2>
              </div>
              {currentDocument ? (
                <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
                  {currentDocument.path}
                </span>
              ) : null}
            </div>
            <pre className="mt-5 overflow-x-auto rounded-2xl border border-white/10 bg-slate-950/80 p-4 text-sm leading-7 text-slate-200">
              {currentDocument?.raw_yaml ?? "No document selected."}
            </pre>
          </section>
        </section>

        <aside className="space-y-6 lg:col-span-4">
          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">validation</p>
                <h2 className="mt-2 text-xl font-semibold text-white">Current issues</h2>
              </div>
              <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
                {workspace.validation.issues.length}
              </span>
            </div>
            {workspace.validation.issues.length === 0 ? (
              <p className="mt-4 rounded-2xl border border-emerald-400/20 bg-emerald-400/10 px-4 py-3 text-sm text-emerald-100">
                No validation issues are currently reported for this workspace.
              </p>
            ) : (
              <div className="mt-5 space-y-3">
                {workspace.validation.issues.map((issue) => (
                  <button
                    key={`${issue.code}-${issue.subject}-${issue.location ?? ""}`}
                    type="button"
                    onClick={() => setSelectedIssueCode(issue.code)}
                    className={`w-full rounded-2xl border px-4 py-3 text-left transition ${
                      activeIssue?.code === issue.code
                        ? issue.severity === "error"
                          ? "border-rose-400/60 bg-rose-400/10 text-rose-50"
                          : "border-amber-400/60 bg-amber-400/10 text-amber-50"
                        : "border-white/10 bg-slate-950/60 text-slate-300 hover:border-sky-400/40 hover:text-white"
                    }`}
                  >
                    <div className="flex items-center justify-between gap-3">
                      <span className="font-medium">{issue.code}</span>
                      <span className="text-xs uppercase tracking-[0.2em] text-slate-500">
                        {issue.severity}
                      </span>
                    </div>
                    <p className="mt-2 text-sm leading-6">{issue.message}</p>
                  </button>
                ))}
              </div>
            )}
            {activeIssue ? <IssueDetail issue={activeIssue} rule={activeRule} /> : null}
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6">
            <p className="text-xs uppercase tracking-[0.3em] text-slate-500">
              What this app is showing
            </p>
            <ul className="mt-4 space-y-3 text-sm leading-7 text-slate-300">
              <li>Tabs follow the same four-layer model as the CLI and checked-in docs.</li>
              <li>
                Document navigation stays grouped by folders so larger specs remain scannable.
              </li>
              <li>Item links jump across layers without leaving the current workspace snapshot.</li>
              <li>
                Validation still explains the current state even when parsing or graph issues exist.
              </li>
            </ul>
          </section>
        </aside>
      </main>
    </div>
  );
}

function firstPopulatedSection(workspace: BrowserWorkspace): SectionKind | null {
  return workspace.sections.find((section) => section.documents.length > 0)?.kind ?? null;
}

function SummaryCard({ label, value }: { label: string; value: string }) {
  return (
    <div className="app-glass rounded-2xl border border-white/10 px-4 py-3 text-sm shadow-lg shadow-slate-950/10">
      <dt className="text-[11px] uppercase tracking-[0.25em] text-slate-500">{label}</dt>
      <dd className="mt-2 break-all font-medium text-slate-100">{value}</dd>
    </div>
  );
}

function CountCard({ label, value }: { label: string; value: number }) {
  return (
    <div className="app-glass rounded-2xl border border-white/10 px-4 py-4 shadow-lg shadow-slate-950/10">
      <p className="text-[11px] uppercase tracking-[0.25em] text-slate-500">{label}</p>
      <p className="mt-2 text-3xl font-semibold text-white">{value}</p>
    </div>
  );
}

function MetaPill({ label, value }: { label: string; value: string }) {
  return (
    <span className="rounded-full border border-white/10 bg-white/5 px-3 py-2 text-xs uppercase tracking-[0.2em] text-slate-300">
      {label}: {value}
    </span>
  );
}

function InfoPanel({
  title,
  content,
  emptyCopy,
}: {
  title: string;
  content: string | null;
  emptyCopy: string;
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/50 p-4">
      <p className="text-xs uppercase tracking-[0.25em] text-slate-500">{title}</p>
      <p className="mt-3 text-sm leading-7 text-slate-200">{content ?? emptyCopy}</p>
    </div>
  );
}

function RelationshipPanel({
  label,
  ids,
  jumpToItem,
}: {
  label: string;
  ids: string[];
  jumpToItem: (id: string) => void;
}) {
  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/50 p-4">
      <p className="text-xs uppercase tracking-[0.25em] text-slate-500">{label}</p>
      {ids.length === 0 ? (
        <p className="mt-3 text-sm text-slate-400">No linked items are declared here.</p>
      ) : (
        <div className="mt-3 flex flex-wrap gap-2">
          {ids.map((id) => (
            <button
              key={id}
              type="button"
              onClick={() => jumpToItem(id)}
              className="rounded-full border border-sky-400/30 bg-sky-400/10 px-3 py-2 text-sm text-sky-100 transition hover:border-sky-300 hover:bg-sky-400/20"
            >
              {id}
            </button>
          ))}
        </div>
      )}
    </div>
  );
}

function TracePanel({ label, groups }: { label: string; groups: BrowserTraceGroup[] }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/50 p-4">
      <div className="flex items-center justify-between gap-3">
        <p className="text-xs uppercase tracking-[0.25em] text-slate-500">{label}</p>
        <span className="text-xs text-slate-500">{groups.length} language blocks</span>
      </div>
      {groups.length === 0 ? (
        <p className="mt-3 text-sm text-slate-400">
          No trace declarations are present for this item.
        </p>
      ) : (
        <div className="mt-4 space-y-3">
          {groups.map((group) => (
            <div
              key={group.language}
              className="rounded-2xl border border-white/10 bg-slate-900/70 p-4"
            >
              <p className="text-sm font-semibold text-white">{group.language}</p>
              <div className="mt-3 space-y-3">
                {group.references.map((reference, index) => (
                  <div
                    key={`${reference.file}-${index}`}
                    className="rounded-2xl border border-white/10 bg-slate-950/70 p-3"
                  >
                    <p className="text-sm font-medium text-slate-100">{reference.file}</p>
                    <p className="mt-2 text-xs uppercase tracking-[0.2em] text-slate-500">
                      symbols
                    </p>
                    <p className="mt-1 text-sm text-slate-300">
                      {reference.symbols.length > 0 ? reference.symbols.join(", ") : "—"}
                    </p>
                    <p className="mt-3 text-xs uppercase tracking-[0.2em] text-slate-500">
                      doc contains
                    </p>
                    <p className="mt-1 text-sm text-slate-300">
                      {reference.doc_contains.length > 0 ? reference.doc_contains.join(", ") : "—"}
                    </p>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}

function IssueDetail({ issue, rule }: { issue: ValidationIssue; rule: ReferencedRule | null }) {
  return (
    <div className="mt-5 rounded-2xl border border-white/10 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.25em] text-slate-500">selected issue</p>
      <h3 className="mt-2 text-lg font-semibold text-white">{issue.code}</h3>
      <p className="mt-3 text-sm leading-7 text-slate-200">{issue.message}</p>
      {issue.location ? (
        <p className="mt-3 text-xs uppercase tracking-[0.2em] text-slate-500">
          location:{" "}
          <span className="normal-case tracking-normal text-slate-300">{issue.location}</span>
        </p>
      ) : null}
      {issue.suggestion ? (
        <div className="mt-4 rounded-2xl border border-sky-400/20 bg-sky-400/10 px-4 py-3 text-sm leading-7 text-sky-50">
          {issue.suggestion}
        </div>
      ) : null}
      {rule ? (
        <div className="mt-4 rounded-2xl border border-white/10 bg-white/5 p-4">
          <p className="text-xs uppercase tracking-[0.25em] text-slate-500">rule reference</p>
          <p className="mt-2 font-medium text-white">{rule.title}</p>
          <p className="mt-2 text-sm leading-7 text-slate-300">{rule.summary}</p>
        </div>
      ) : null}
    </div>
  );
}

export default App;
