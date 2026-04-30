// FEAT-APP-001

import { useCallback, useEffect, useMemo, useState } from "react";
import type {
  AppDataErrorResponse,
  AppPayload,
  BrowserDocument,
  BrowserTraceGroup,
  BrowserWorkspace,
  ReferencedRule,
  SectionKind,
  ValidationIssue,
} from "./types";

type WasmModule = {
  default: () => Promise<void>;
  build_browser_workspace_from_js: (payload: AppPayload) => BrowserWorkspace;
};

type VersionPayload = {
  snapshot: string;
};

type SectionSummary = {
  kind: SectionKind;
  label: string;
  documentCount: number;
  itemCount: number;
};

type SearchResult = {
  id: string;
  title: string;
  kind: SectionKind;
};

type SearchFilter = "all" | SectionKind;

const SECTION_ORDER: SectionKind[] = ["philosophy", "policies", "requirements", "features"];
const SEARCH_RESULTS_LIST_ID = "spec-search-results-list";
const SEARCH_FILTER_GROUP_ID = "spec-search-filter-group";
const REFRESH_POLL_MIN_MS = 2_000;
const REFRESH_POLL_MAX_MS = 10_000;

const SECTION_COPY: Record<SectionKind, string> = {
  philosophy: "Project intent and enduring values.",
  policies: "Repository-wide rules that operationalize philosophy.",
  features: "Implemented capabilities mapped to delivery evidence.",
  requirements: "Specific obligations with traceable ownership.",
};

const ONBOARDING_STORAGE_KEY = "syu-onboarding-dismissed";
const SEARCH_RESULT_LIMIT = 20;
const SEARCH_SHORTCUT_KEY_CLASS_NAME =
  "inline-flex items-center rounded-md border border-white/10 bg-white/5 px-1.5 py-0.5 font-mono text-[10px] text-slate-300";
const SEARCH_FILTER_OPTIONS: Array<{ value: SearchFilter; label: string }> = [
  { value: "all", label: "All" },
  { value: "philosophy", label: "Philosophy" },
  { value: "policies", label: "Policies" },
  { value: "requirements", label: "Requirements" },
  { value: "features", label: "Features" },
];

function searchResultOptionId(id: string, index: number) {
  return `spec-search-result-${index}-${id.toLowerCase().replace(/[^a-z0-9_-]/g, "-")}`;
}

function App() {
  const [workspace, setWorkspace] = useState<BrowserWorkspace | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [selectedSection, setSelectedSection] = useState<SectionKind>("philosophy");
  const [selectedDocumentPath, setSelectedDocumentPath] = useState("");
  const [selectedItemId, setSelectedItemId] = useState("");
  const [selectedIssueKey, setSelectedIssueKey] = useState<string | null>(null);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchFilter, setSearchFilter] = useState<SearchFilter>("all");
  const [focusedResultIndex, setFocusedResultIndex] = useState(-1);
  const [showOnboarding, setShowOnboarding] = useState(() => shouldShowOnboarding());
  const [navigationHistory, setNavigationHistory] = useState<string[]>([]);
  const [snapshotVersion, setSnapshotVersion] = useState<string | null>(null);
  const [isRefreshing, setIsRefreshing] = useState(false);
  const [refreshError, setRefreshError] = useState<string | null>(null);
  const [lastSuccessfulRefreshAt, setLastSuccessfulRefreshAt] = useState<string | null>(null);

  const applyWorkspace = useCallback((browserWorkspace: BrowserWorkspace) => {
    setWorkspace(browserWorkspace);
    setShowOnboarding(shouldShowOnboarding(browserWorkspace.workspace_root));

    const hash = window.location.hash.replace(/^#\/?/, "");
    const hashParts = hash.split("/");
    const hashSection = hashParts[0] ?? "";
    const hashItemId = hashParts[1] ?? "";
    const hashTarget =
      hashItemId && isSectionKind(hashSection) ? browserWorkspace.item_index.get(hashItemId) : null;

    if (hashTarget && hashItemId) {
      setSelectedSection(hashTarget.kind);
      setSelectedDocumentPath(hashTarget.document_path);
      setSelectedItemId(hashItemId);
    } else if (isSectionKind(hashSection)) {
      const section = browserWorkspace.sections.find((s) => s.kind === hashSection);
      setSelectedSection(hashSection);
      setSelectedDocumentPath(section?.documents[0]?.path ?? "");
      setSelectedItemId(section?.documents[0]?.items[0]?.id ?? "");
    } else {
      const nextSection = firstPopulatedSection(browserWorkspace) ?? "philosophy";
      setSelectedSection(nextSection);
      const firstDocument = browserWorkspace.sections.find(
        (section) => section.kind === nextSection,
      )?.documents[0];
      setSelectedDocumentPath(firstDocument?.path ?? "");
      setSelectedItemId(firstDocument?.items[0]?.id ?? "");
    }

    setSelectedIssueKey((current) => {
      if (
        current &&
        browserWorkspace.validation.issues.some(
          (issue) => validationIssueSelectionKey(issue) === current,
        )
      ) {
        return current;
      }

      const firstIssue = browserWorkspace.validation.issues[0];
      return firstIssue ? validationIssueSelectionKey(firstIssue) : null;
    });
  }, []);

  const loadWorkspace = useCallback(
    async (mode: "initial" | "refresh" = "initial") => {
      const refreshing = mode === "refresh";
      if (refreshing) {
        setIsRefreshing(true);
      }

      try {
        const [wasmModule, dataResponse] = await Promise.all([
          import("./wasm/syu_app_wasm.js") as Promise<WasmModule>,
          fetch("/api/app-data.json", { cache: "no-store" }),
        ]);

        if (!dataResponse.ok) {
          throw new Error(await describeAppDataRefreshFailure(dataResponse));
        }
        const snapshot = dataResponse.headers.get("x-syu-snapshot");
        if (!snapshot) {
          throw new Error("Failed to load app snapshot header");
        }

        const payload = (await dataResponse.json()) as AppPayload;
        await wasmModule.default();
        const browserWorkspace = wasmModule.build_browser_workspace_from_js(payload);

        setError(null);
        setRefreshError(null);
        setSnapshotVersion(snapshot);
        setLastSuccessfulRefreshAt(new Date().toISOString());
        applyWorkspace(browserWorkspace);
      } catch (loadError) {
        if (refreshing) {
          // eslint-disable-next-line no-console
          console.error("Failed to refresh syu app workspace", loadError);
          setRefreshError(formatRefreshFailure("reload the workspace snapshot", loadError));
        } else {
          setError(errorMessage(loadError, "Failed to load syu app"));
        }
      } finally {
        setLoading(false);
        if (refreshing) {
          setIsRefreshing(false);
        }
      }
    },
    [applyWorkspace],
  );

  useEffect(() => {
    void loadWorkspace();
  }, [loadWorkspace]);

  useEffect(() => {
    if (snapshotVersion == null) {
      return;
    }

    let cancelled = false;
    let stablePollCount = 0;
    let timeoutId: number | null = null;

    const currentDelay = () =>
      Math.min(REFRESH_POLL_MAX_MS, REFRESH_POLL_MIN_MS * 2 ** stablePollCount);

    const schedulePoll = (delay: number) => {
      if (cancelled) {
        return;
      }
      if (timeoutId != null) {
        window.clearTimeout(timeoutId);
      }
      timeoutId = window.setTimeout(() => {
        void pollVersion();
      }, delay);
    };

    const pollVersion = async () => {
      if (cancelled) {
        return;
      }
      if (document.hidden || isRefreshing) {
        schedulePoll(currentDelay());
        return;
      }

      try {
        const response = await fetch("/api/version", { cache: "no-store" });
        if (!response.ok) {
          throw new Error(`Failed to poll app version: ${response.status} ${response.statusText}`);
        }
        const nextVersion = (await response.json()) as VersionPayload;
        if (!cancelled) {
          setRefreshError(null);
        }
        if (!cancelled && nextVersion.snapshot !== snapshotVersion) {
          stablePollCount = 0;
          await loadWorkspace("refresh");
          schedulePoll(REFRESH_POLL_MIN_MS);
          return;
        }

        stablePollCount = Math.min(stablePollCount + 1, 3);
        schedulePoll(currentDelay());
      } catch (pollError) {
        stablePollCount = 0;
        if (!cancelled) {
          // eslint-disable-next-line no-console
          console.error("Failed to poll app version for refresh", pollError);
          setRefreshError(formatRefreshFailure("check for workspace updates", pollError));
        }
        schedulePoll(REFRESH_POLL_MIN_MS);
      }
    };

    const handleVisibilityChange = () => {
      if (cancelled) {
        return;
      }
      if (!document.hidden) {
        stablePollCount = 0;
        schedulePoll(REFRESH_POLL_MIN_MS);
      }
    };

    document.addEventListener("visibilitychange", handleVisibilityChange);
    schedulePoll(REFRESH_POLL_MIN_MS);

    return () => {
      cancelled = true;
      document.removeEventListener("visibilitychange", handleVisibilityChange);
      if (timeoutId != null) {
        window.clearTimeout(timeoutId);
        timeoutId = null;
      }
    };
  }, [isRefreshing, loadWorkspace, snapshotVersion]);

  useEffect(() => {
    setFocusedResultIndex(-1);
  }, [workspace, searchFilter, searchQuery]);

  const triggerRefresh = useCallback(() => {
    void loadWorkspace("refresh");
  }, [loadWorkspace]);

  const sectionSummaries = useMemo(() => {
    if (!workspace) {
      return [] as SectionSummary[];
    }

    return SECTION_ORDER.map((kind) => {
      const section = workspace.sections.find((candidate) => candidate.kind === kind);
      return {
        kind,
        label: section?.label ?? kind,
        documentCount: section?.documents.length ?? 0,
        itemCount:
          section?.documents.reduce((total, document) => total + document.items.length, 0) ?? 0,
      };
    });
  }, [workspace]);

  const currentSection = useMemo(() => {
    return workspace?.sections.find((section) => section.kind === selectedSection) ?? null;
  }, [selectedSection, workspace]);

  const currentSectionSummary = useMemo(() => {
    return sectionSummaries.find((summary) => summary.kind === selectedSection) ?? null;
  }, [sectionSummaries, selectedSection]);

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

  const refreshState = refreshError ? "stale" : isRefreshing ? "refreshing" : "current";
  const refreshStateClasses =
    refreshState === "stale"
      ? "border-rose-400/40 bg-rose-400/10 text-rose-100"
      : refreshState === "refreshing"
        ? "border-amber-400/40 bg-amber-400/10 text-amber-100"
        : "border-emerald-400/40 bg-emerald-400/10 text-emerald-100";
  const refreshStateLabel =
    refreshState === "stale"
      ? "Stale snapshot"
      : refreshState === "refreshing"
        ? "Refreshing…"
        : "Current";
  const lastRefreshLabel = formatRefreshTimestamp(lastSuccessfulRefreshAt);

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

  const activeIssueIndex = useMemo(() => {
    if (!workspace || workspace.validation.issues.length === 0) {
      return null;
    }

    if (selectedIssueKey) {
      const selectedIndex = workspace.validation.issues.findIndex(
        (issue) => validationIssueSelectionKey(issue) === selectedIssueKey,
      );
      if (selectedIndex >= 0) {
        return selectedIndex;
      }
    }

    return 0;
  }, [selectedIssueKey, workspace]);

  const activeIssue = useMemo(() => {
    if (!workspace || activeIssueIndex == null) {
      return null;
    }
    return workspace.validation.issues[activeIssueIndex] ?? null;
  }, [activeIssueIndex, workspace]);

  const activeRule = useMemo(() => {
    if (!workspace || !activeIssue) {
      return null;
    }

    return (
      workspace.validation.referenced_rules.find((rule) => rule.code === activeIssue.code) ?? null
    );
  }, [activeIssue, workspace]);

  const infoPanels = useMemo(() => {
    if (!currentItem) {
      return [] as Array<{ title: string; content: string }>;
    }

    return [
      { title: "Summary", content: currentItem.summary },
      { title: "Description", content: currentItem.description },
      { title: "Product design principle", content: currentItem.product_design_principle },
      { title: "Coding guideline", content: currentItem.coding_guideline },
    ].filter((panel): panel is { title: string; content: string } => Boolean(panel.content));
  }, [currentItem]);

  const relationshipPanels = useMemo(() => {
    if (!currentItem) {
      return [] as Array<{ label: string; ids: string[] }>;
    }

    return [
      { label: "Linked philosophies", ids: currentItem.linked_philosophies },
      { label: "Linked policies", ids: currentItem.linked_policies },
      { label: "Linked requirements", ids: currentItem.linked_requirements },
      { label: "Linked features", ids: currentItem.linked_features },
    ].filter((panel) => panel.ids.length > 0);
  }, [currentItem]);

  const maxSectionItems = useMemo(() => {
    return Math.max(1, ...sectionSummaries.map((summary) => summary.itemCount));
  }, [sectionSummaries]);

  const searchState = useMemo(() => {
    const trimmed = searchQuery.trim().toLowerCase();
    if (!workspace || trimmed.length === 0) {
      return {
        results: [] as SearchResult[],
        hasMore: false,
      };
    }
    const results: SearchResult[] = [];
    for (const section of workspace.sections) {
      for (const document of section.documents) {
        for (const item of document.items) {
          if (
            item.id.toLowerCase().includes(trimmed) ||
            item.title.toLowerCase().includes(trimmed) ||
            (item.summary?.toLowerCase().includes(trimmed) ?? false) ||
            (item.description?.toLowerCase().includes(trimmed) ?? false)
          ) {
            if (searchFilter !== "all" && item.kind !== searchFilter) {
              continue;
            }
            results.push({ id: item.id, title: item.title, kind: item.kind });
          }
        }
      }
    }
    return {
      results: results.slice(0, SEARCH_RESULT_LIMIT),
      hasMore: results.length > SEARCH_RESULT_LIMIT,
    };
  }, [searchFilter, workspace, searchQuery]);
  const searchResults = searchState.results;
  const activeSearchResultId = useMemo(() => {
    if (focusedResultIndex < 0 || focusedResultIndex >= searchResults.length) {
      return undefined;
    }

    return searchResultOptionId(searchResults[focusedResultIndex].id, focusedResultIndex);
  }, [focusedResultIndex, searchResults]);
  const hasSearchResultsList = searchQuery.trim().length > 0 && searchResults.length > 0;

  useEffect(() => {
    if (loading || !workspace) {
      return;
    }

    const target = selectedItemId ? workspace.item_index.get(selectedItemId) : null;
    const nextHash = target ? `#${target.kind}/${target.id}` : `#${selectedSection}`;

    if (window.location.hash !== nextHash) {
      history.replaceState(null, "", nextHash);
    }
  }, [loading, selectedItemId, selectedSection, workspace]);

  const sectionIssueSummaries = useMemo(() => {
    const result = new Map<SectionKind, { count: number; hasError: boolean }>();
    for (const kind of SECTION_ORDER) {
      result.set(kind, { count: 0, hasError: false });
    }
    if (!workspace) {
      return result;
    }
    for (const issue of workspace.validation.issues) {
      const target = workspace.item_index.get(issue.subject);
      if (!target) {
        continue;
      }
      const entry = result.get(target.kind);
      if (!entry) {
        continue;
      }
      entry.count += 1;
      if (issue.severity === "error") {
        entry.hasError = true;
      }
    }
    return result;
  }, [workspace]);

  const sectionIssueCount = useMemo(() => {
    return sectionIssueSummaries.get(selectedSection)?.count ?? 0;
  }, [sectionIssueSummaries, selectedSection]);

  const sectionIssueHasError = useMemo(() => {
    return sectionIssueSummaries.get(selectedSection)?.hasError ?? false;
  }, [sectionIssueSummaries, selectedSection]);

  const requirementTraceRatio = useMemo(() => {
    if (!workspace) {
      return 0;
    }

    return ratio(
      workspace.validation.trace_summary.requirement_traces.validated,
      workspace.validation.trace_summary.requirement_traces.declared,
    );
  }, [workspace]);

  const featureTraceRatio = useMemo(() => {
    if (!workspace) {
      return 0;
    }

    return ratio(
      workspace.validation.trace_summary.feature_traces.validated,
      workspace.validation.trace_summary.feature_traces.declared,
    );
  }, [workspace]);

  const selectSection = (nextSection: SectionKind) => {
    if (!workspace) {
      return;
    }

    const section = workspace.sections.find((candidate) => candidate.kind === nextSection);
    setSelectedSection(nextSection);
    setSelectedDocumentPath(section?.documents[0]?.path ?? "");
    setSelectedItemId(section?.documents[0]?.items[0]?.id ?? "");
  };

  const resetNavigation = () => {
    if (!workspace) {
      return;
    }

    setSearchQuery("");
    setNavigationHistory([]);
    const nextSection = firstPopulatedSection(workspace) ?? "philosophy";
    selectSection(nextSection);
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

    if (selectedItemId && selectedItemId !== id) {
      setNavigationHistory((prev) => [...prev.slice(-4), selectedItemId]);
    }

    setSelectedSection(target.kind);
    setSelectedDocumentPath(target.document_path);
    setSelectedItemId(id);
  };

  const dismissOnboarding = () => {
    setShowOnboarding(false);
    persistOnboardingDismissal(workspace?.workspace_root);
  };

  const goBack = () => {
    const prevId = navigationHistory[navigationHistory.length - 1];
    if (!prevId || !workspace) {
      return;
    }
    const target = workspace.item_index.get(prevId);
    if (!target) {
      return;
    }
    setNavigationHistory((h) => h.slice(0, -1));
    setSelectedSection(target.kind);
    setSelectedDocumentPath(target.document_path);
    setSelectedItemId(prevId);
  };

  const handleSearchSelect = (id: string) => {
    setSearchQuery("");
    jumpToItem(id);
  };

  if (loading) {
    return (
      <div className="app-shell flex items-center justify-center px-6 text-slate-300">
        <div className="app-glass rounded-3xl border border-sky-400/20 px-6 py-5 shadow-2xl shadow-sky-950/30">
          Loading syu...
        </div>
      </div>
    );
  }

  if (error) {
    return (
      <div className="app-shell flex items-center justify-center px-6 text-slate-100">
        <div className="app-glass max-w-2xl rounded-3xl border border-rose-500/40 px-8 py-6 shadow-2xl shadow-rose-950/30">
          <div className="text-sm font-semibold tracking-wide text-slate-400">syu</div>
          <h1 className="mt-1 text-2xl font-semibold">Workspace could not load</h1>
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
      <header className="sticky top-0 z-20 border-b border-white/10 bg-slate-950/90 backdrop-blur-2xl">
        <div className="mx-auto flex max-w-7xl flex-col gap-4 px-4 py-4 sm:px-6 md:flex-row md:items-center md:justify-between md:px-8">
          <div className="flex items-center justify-between gap-4 md:min-w-0">
            <h1 className="text-2xl font-semibold tracking-tight text-white">
              <button
                type="button"
                onClick={resetNavigation}
                className="transition hover:text-sky-300"
                aria-label="syu — go to first item"
              >
                syu
              </button>
            </h1>
            <div className="flex items-center gap-3 md:hidden">
              <div className="text-right text-[11px] text-slate-300">
                <div className="font-medium text-slate-100">Last refreshed</div>
                <time
                  aria-label="Last successful refresh"
                  className="block text-slate-400"
                  dateTime={lastSuccessfulRefreshAt ?? ""}
                  title={lastSuccessfulRefreshAt ?? undefined}
                >
                  {lastRefreshLabel}
                </time>
              </div>
              <span
                className={`inline-flex rounded-full border px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] ${refreshStateClasses}`}
              >
                {refreshStateLabel}
              </span>
              <button
                type="button"
                onClick={triggerRefresh}
                disabled={isRefreshing}
                className="inline-flex items-center rounded-full border border-sky-400/30 bg-sky-400/10 px-3 py-1.5 text-xs font-semibold text-sky-100 transition hover:bg-sky-400/20 disabled:cursor-wait disabled:opacity-60"
              >
                Refresh now
              </button>
            </div>
          </div>
          <nav
            aria-label="Top level sections"
            className="flex gap-2 overflow-x-auto whitespace-nowrap pb-1 [-ms-overflow-style:none] [scrollbar-width:none] [&::-webkit-scrollbar]:hidden"
          >
            {SECTION_ORDER.map((section) => {
              const isActive = section === selectedSection;
              const issueSummary = sectionIssueSummaries.get(section);
              const issueCount = issueSummary?.count ?? 0;
              const issueHasError = issueSummary?.hasError ?? false;
              return (
                <button
                  key={section}
                  type="button"
                  onClick={() => selectSection(section)}
                  className={`inline-flex items-center gap-1.5 rounded-full border px-4 py-2 text-sm font-medium capitalize transition ${
                    isActive
                      ? "border-sky-400 bg-sky-400/20 text-sky-50"
                      : "border-white/10 bg-white/5 text-slate-300 hover:border-sky-400/40 hover:text-white"
                  }`}
                >
                  {section}
                  {issueCount > 0 && (
                    <span
                      className={`rounded-full px-1.5 py-0.5 text-xs font-semibold leading-none ${
                        issueHasError
                          ? "bg-rose-500/80 text-rose-50"
                          : "bg-amber-500/80 text-amber-50"
                      }`}
                      aria-label={`${issueCount} ${issueHasError ? "error" : "warning"}${issueCount === 1 ? "" : "s"}`}
                    >
                      {issueCount}
                    </span>
                  )}
                </button>
              );
            })}
          </nav>
          <div className="hidden shrink-0 items-center gap-3 md:flex">
            <div className="text-right text-xs text-slate-300">
              <div className="font-medium text-slate-100">Last refreshed</div>
              <time
                aria-label="Last successful refresh"
                className="block text-slate-400"
                dateTime={lastSuccessfulRefreshAt ?? ""}
                title={lastSuccessfulRefreshAt ?? undefined}
              >
                {lastRefreshLabel}
              </time>
            </div>
            <span
              className={`inline-flex rounded-full border px-3 py-1 text-xs font-semibold uppercase tracking-[0.2em] ${refreshStateClasses}`}
            >
              {refreshStateLabel}
            </span>
            <button
              type="button"
              onClick={triggerRefresh}
              disabled={isRefreshing}
              className="inline-flex items-center rounded-full border border-sky-400/30 bg-sky-400/10 px-4 py-2 text-sm font-semibold text-sky-100 transition hover:bg-sky-400/20 disabled:cursor-wait disabled:opacity-60"
            >
              Refresh now
            </button>
          </div>
        </div>
      </header>

      <main className="mx-auto grid max-w-7xl gap-6 px-4 py-6 sm:px-6 md:grid-cols-[18rem_minmax(0,1fr)] md:px-8">
        {refreshError && (
          <div
            role="alert"
            className="md:col-span-2 rounded-3xl border border-rose-400/30 bg-rose-400/10 px-5 py-4 text-sm text-rose-50 shadow-2xl shadow-rose-950/10"
          >
            <p className="font-semibold">Live refresh needs attention.</p>
            <p className="mt-2 leading-7 text-rose-100">
              Showing the last successfully loaded workspace snapshot while `syu app` recovers. Fix
              the workspace or keep this tab open until the next refresh succeeds.
            </p>
            <div className="mt-3 flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
              <p className="text-xs text-rose-200/90">
                Last successful refresh:{" "}
                <time
                  aria-label="Last successful refresh"
                  dateTime={lastSuccessfulRefreshAt ?? ""}
                  title={lastSuccessfulRefreshAt ?? undefined}
                >
                  {lastRefreshLabel}
                </time>
              </p>
              <button
                type="button"
                onClick={triggerRefresh}
                disabled={isRefreshing}
                className="inline-flex items-center justify-center rounded-full border border-rose-200/30 bg-rose-200/10 px-4 py-2 text-sm font-semibold text-rose-50 transition hover:bg-rose-200/20 disabled:cursor-wait disabled:opacity-60"
              >
                Refresh now
              </button>
            </div>
            <p className="mt-2 break-words text-xs text-rose-200/90">{refreshError}</p>
          </div>
        )}
        {isRefreshing && (
          <div className="md:col-span-2 rounded-3xl border border-amber-400/30 bg-amber-400/10 px-5 py-4 text-sm text-amber-100 shadow-2xl shadow-amber-950/10">
            Refreshing workspace data...
          </div>
        )}
        {workspace.app_server.remotely_reachable && (
          <div
            role="alert"
            className="md:col-span-2 rounded-3xl border border-amber-400/40 bg-amber-400/10 px-5 py-4 text-sm text-amber-50 shadow-2xl shadow-amber-950/10"
          >
            <p className="font-semibold">Remote access is enabled for this session.</p>
            <p className="mt-2 leading-7 text-amber-100">
              This browser UI is reachable at{" "}
              <span className="font-mono text-amber-50">
                {formatAppServerUrl(workspace.app_server.bind, workspace.app_server.port)}
              </span>{" "}
              because <code className="rounded bg-black/20 px-1 py-0.5">--allow-remote</code> was
              used intentionally when{" "}
              <code className="rounded bg-black/20 px-1 py-0.5">syu app</code> started.
            </p>
            <p className="mt-2 text-xs text-amber-200/90">
              Use <code className="rounded bg-black/20 px-1 py-0.5">--bind 127.0.0.1</code> next
              time to keep the workspace view local to this machine.
            </p>
          </div>
        )}
        {showOnboarding && (
          <div className="md:col-span-2 flex items-start justify-between gap-4 rounded-3xl border border-sky-400/30 bg-sky-400/10 px-5 py-4 text-sm leading-7 text-sky-100 shadow-2xl shadow-sky-950/15">
            <p>
              <span className="font-semibold">Welcome to syu.</span> Browse your specification
              across four layers:{" "}
              <span className="text-sky-300">Philosophy → Policies → Requirements → Features</span>.
              Click any item to explore its traces and validation status.
            </p>
            <button
              type="button"
              onClick={dismissOnboarding}
              aria-label="Dismiss welcome banner"
              className="shrink-0 rounded-full border border-sky-400/30 bg-sky-400/10 px-2 py-1 text-sky-300 transition hover:bg-sky-400/20"
            >
              ×
            </button>
          </div>
        )}
        <aside className="space-y-5">
          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15">
            <p className="text-xs uppercase tracking-[0.3em] text-slate-500">workspace</p>
            <p
              className="mt-3 truncate text-sm font-medium text-slate-100"
              title={workspace.workspace_root}
              aria-label={workspace.workspace_root}
            >
              {truncatePath(workspace.workspace_root)}
            </p>
            <p className="mt-2 break-all text-sm text-slate-400">
              spec root: {workspace.spec_root}
            </p>
            <div className="mt-4 flex flex-wrap items-center gap-3 text-xs">
              <span
                className={`inline-flex rounded-full border px-3 py-1 font-semibold uppercase tracking-[0.2em] ${refreshStateClasses}`}
              >
                {refreshStateLabel}
              </span>
              <p className="text-slate-400">
                Last successful refresh:{" "}
                <time
                  aria-label="Last successful refresh"
                  className="text-slate-300"
                  dateTime={lastSuccessfulRefreshAt ?? ""}
                  title={lastSuccessfulRefreshAt ?? undefined}
                >
                  {lastRefreshLabel}
                </time>
              </p>
            </div>
            <div className="mt-5 grid gap-3 sm:grid-cols-3 lg:grid-cols-1">
              <CompactMetric
                label="issues"
                value={`${workspace.validation.issues.length}`}
                note={workspace.validation.issues.length === 1 ? "open issue" : "open issues"}
              />
              <CompactMetric
                label="requirement traces"
                value={`${workspace.validation.trace_summary.requirement_traces.validated}/${workspace.validation.trace_summary.requirement_traces.declared}`}
                note="validated / declared"
                tone="sky"
                ratio={requirementTraceRatio}
                hint={{
                  label: "Requirement traces",
                  description:
                    "Declared traces are the requirement test references written in the spec. Validated traces are the declared references that syu could confirm in the current workspace. A gap means some declared requirement traces are stale or unresolved.",
                }}
              />
              <CompactMetric
                label="feature traces"
                value={`${workspace.validation.trace_summary.feature_traces.validated}/${workspace.validation.trace_summary.feature_traces.declared}`}
                note="validated / declared"
                tone="violet"
                ratio={featureTraceRatio}
                hint={{
                  label: "Feature traces",
                  description:
                    "Declared traces are the feature implementation references written in the spec. Validated traces are the declared references that syu could confirm in the current workspace. A gap means some declared feature traces are stale or unresolved.",
                }}
              />
            </div>
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-4 shadow-2xl shadow-sky-950/15">
            <label htmlFor="spec-search" className="sr-only">
              Search spec items
            </label>
            <div className="relative">
              <svg
                aria-hidden="true"
                className="pointer-events-none absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-slate-500"
                fill="none"
                viewBox="0 0 24 24"
                stroke="currentColor"
                strokeWidth={2}
              >
                <path
                  strokeLinecap="round"
                  strokeLinejoin="round"
                  d="M21 21l-4.35-4.35M17 11A6 6 0 1 1 5 11a6 6 0 0 1 12 0z"
                />
              </svg>
              <input
                id="spec-search"
                type="search"
                role="combobox"
                aria-autocomplete="list"
                aria-describedby="spec-search-shortcuts-description"
                aria-controls={hasSearchResultsList ? SEARCH_RESULTS_LIST_ID : undefined}
                aria-expanded={hasSearchResultsList}
                aria-activedescendant={hasSearchResultsList ? activeSearchResultId : undefined}
                placeholder={`Search items by ID or keyword (up to ${SEARCH_RESULT_LIMIT} matches)…`}
                value={searchQuery}
                onChange={(e) => {
                  setSearchQuery(e.target.value);
                  setFocusedResultIndex(-1);
                }}
                onKeyDown={(e) => {
                  if (e.key === "ArrowDown") {
                    if (searchResults.length === 0) {
                      return;
                    }
                    e.preventDefault();
                    setFocusedResultIndex((prev) => Math.min(prev + 1, searchResults.length - 1));
                  } else if (e.key === "ArrowUp") {
                    if (searchResults.length === 0) {
                      return;
                    }
                    e.preventDefault();
                    setFocusedResultIndex((prev) => Math.max(prev - 1, -1));
                  } else if (e.key === "Enter") {
                    const focusedResult =
                      focusedResultIndex >= 0 && focusedResultIndex < searchResults.length
                        ? searchResults[focusedResultIndex]
                        : null;

                    if (focusedResult) {
                      handleSearchSelect(focusedResult.id);
                    } else if (searchResults.length === 1) {
                      handleSearchSelect(searchResults[0].id);
                    }
                  } else if (e.key === "Escape") {
                    setSearchQuery("");
                    setSearchFilter("all");
                    setFocusedResultIndex(-1);
                  }
                }}
                className="w-full rounded-2xl border border-white/10 bg-slate-900/60 py-2 pl-9 pr-4 text-sm text-slate-100 placeholder-slate-500 focus:border-sky-400/60 focus:outline-none focus:ring-1 focus:ring-sky-400/40"
              />
            </div>
            <p id="spec-search-shortcuts-description" className="sr-only">
              Keyboard shortcuts: ArrowDown and ArrowUp move through results, Enter opens the
              highlighted or only match, and Escape clears the search.
            </p>
            <div
              id="spec-search-shortcuts-panel"
              role="note"
              className="mt-3 rounded-2xl border border-sky-400/20 bg-sky-400/10 px-3 py-3 text-sm text-sky-50"
            >
              <p className="text-xs font-semibold uppercase tracking-[0.2em] text-sky-200">
                Search shortcuts
              </p>
              <p className="mt-1 text-sm text-sky-100">
                Keep focus in the search box and use the keyboard to move through results.
              </p>
              <p className="mt-2 flex flex-wrap items-center gap-1.5 text-sm text-sky-100">
                <kbd className={SEARCH_SHORTCUT_KEY_CLASS_NAME}>ArrowDown</kbd>
                <span>next result</span>
                <kbd className={SEARCH_SHORTCUT_KEY_CLASS_NAME}>ArrowUp</kbd>
                <span>previous result</span>
                <kbd className={SEARCH_SHORTCUT_KEY_CLASS_NAME}>Enter</kbd>
                <span>open the highlighted or only match</span>
                <kbd className={SEARCH_SHORTCUT_KEY_CLASS_NAME}>Escape</kbd>
                <span>clear the search</span>
              </p>
            </div>
            <p className="mt-2 text-xs text-slate-400">
              Search shows up to {SEARCH_RESULT_LIMIT} matches at a time. Filter by layer or refine
              broad queries for a narrower result list.
            </p>
            <div
              id={SEARCH_FILTER_GROUP_ID}
              role="group"
              aria-label="Search layer filters"
              className="mt-3 flex flex-wrap gap-2"
            >
              {SEARCH_FILTER_OPTIONS.map((option) => {
                const active = searchFilter === option.value;
                return (
                  <button
                    key={option.value}
                    type="button"
                    aria-pressed={active}
                    onClick={() => {
                      setSearchFilter(option.value);
                      setFocusedResultIndex(-1);
                    }}
                    className={`rounded-full border px-3 py-1.5 text-xs font-medium transition ${
                      active
                        ? "border-sky-400/60 bg-sky-400/20 text-sky-50"
                        : "border-white/10 bg-white/5 text-slate-300 hover:border-sky-400/40 hover:text-white"
                    }`}
                  >
                    {option.label}
                  </button>
                );
              })}
            </div>
            {searchQuery.trim().length > 0 && (
              <div className="mt-3 space-y-1">
                {searchResults.length === 0 ? (
                  <p className="px-2 py-2 text-xs text-slate-500" role="status">
                    No items match{searchFilter === "all" ? "." : ` in ${searchFilter}.`}
                  </p>
                ) : (
                  <div
                    id={SEARCH_RESULTS_LIST_ID}
                    role="listbox"
                    aria-label="Search results"
                    className="space-y-1"
                  >
                    {searchResults.map((result, index) => (
                      <div
                        key={result.id}
                        id={searchResultOptionId(result.id, index)}
                        role="option"
                        onMouseDown={(e) => e.preventDefault()}
                        onMouseEnter={() => setFocusedResultIndex(index)}
                        onClick={() => handleSearchSelect(result.id)}
                        className={`flex cursor-pointer items-start gap-2 rounded-xl border px-3 py-2 text-left transition hover:border-sky-400/40 hover:bg-sky-400/10 ${
                          index === focusedResultIndex
                            ? "border-sky-400/60 bg-white/5 ring-2 ring-sky-400"
                            : "border-white/5 bg-white/5"
                        }`}
                      >
                        <span className="min-w-0 flex-1">
                          <span className="block truncate text-xs font-semibold text-sky-300">
                            {result.id}
                          </span>
                          <span className="block truncate text-xs text-slate-400">
                            {result.title}
                          </span>
                        </span>
                        <span className="shrink-0 rounded-full border border-white/10 bg-white/5 px-1.5 py-0.5 text-[10px] capitalize text-slate-500">
                          {result.kind}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
                {searchState.hasMore && (
                  <p className="px-2 py-1 text-[11px] text-slate-500">
                    Showing the first {SEARCH_RESULT_LIMIT}{" "}
                    {searchFilter === "all" ? "matches" : `${searchFilter} matches`} — refine your
                    query for fewer results.
                  </p>
                )}
              </div>
            )}
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">
                  layered navigation
                </p>
                <h2 className="mt-2 text-lg font-semibold text-white">Sections</h2>
              </div>
              <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
                4 layers
              </span>
            </div>
            <div className="mt-4 space-y-3">
              {sectionSummaries.map((summary) => (
                <LayerNavigationCard
                  key={summary.kind}
                  summary={summary}
                  active={summary.kind === selectedSection}
                  maxItems={maxSectionItems}
                  onSelect={() => selectSection(summary.kind)}
                />
              ))}
            </div>
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15">
            <div className="flex items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">
                  section drilldown
                </p>
                <h2 className="mt-2 text-lg font-semibold capitalize text-white">
                  {selectedSection}
                </h2>
              </div>
              <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
                {(currentSection?.documents.length ?? 0) === 1
                  ? "1 doc"
                  : `${currentSection?.documents.length ?? 0} docs`}
              </span>
            </div>
            <p className="mt-3 text-sm leading-6 text-slate-400">{SECTION_COPY[selectedSection]}</p>
            {!currentSection || currentSection.documents.length === 0 ? (
              <div className="mt-4 rounded-2xl border border-dashed border-white/10 px-4 py-4 text-sm text-slate-400">
                No documents were discovered for this layer.
              </div>
            ) : currentSection.documents.length === 1 && currentDocument ? (
              <div className="mt-4 rounded-2xl border border-white/10 bg-slate-950/60 px-4 py-4">
                <p className="text-sm font-medium text-white">{currentDocument.title}</p>
                <p className="mt-1 text-xs text-slate-500">{currentDocument.path}</p>
              </div>
            ) : (
              <div className="mt-4 space-y-3">
                {documentGroups.map(([group, documents]) => (
                  <div
                    key={group}
                    className="rounded-2xl border border-white/10 bg-slate-950/60 p-3"
                  >
                    <p className="text-[11px] font-semibold uppercase tracking-[0.25em] text-slate-500">
                      {group}
                    </p>
                    <div className="mt-3 space-y-2">
                      {documents.map((document) => {
                        const isActive = currentDocument?.path === document.path;
                        return (
                          <button
                            key={document.path}
                            type="button"
                            onClick={() => selectDocument(document)}
                            className={`w-full rounded-2xl border px-3 py-3 text-left transition ${
                              isActive
                                ? "border-sky-400/60 bg-sky-400/15 text-sky-50"
                                : "border-white/10 bg-white/5 text-slate-300 hover:border-sky-400/40 hover:text-white"
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
                ))}
              </div>
            )}
          </section>
        </aside>

        <section className="space-y-6">
          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6">
            <div className="flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">selected layer</p>
                <h2 className="mt-2 text-2xl font-semibold capitalize text-white">
                  {currentSection?.label ?? selectedSection}
                </h2>
                <p className="mt-2 text-sm leading-7 text-slate-400">
                  {SECTION_COPY[selectedSection]}
                </p>
              </div>
              <div className="grid gap-3 sm:grid-cols-3">
                <SummaryStat
                  label="documents"
                  value={`${currentSectionSummary?.documentCount ?? 0}`}
                />
                <SummaryStat label="items" value={`${currentSectionSummary?.itemCount ?? 0}`} />
                <SummaryStat
                  label="issues"
                  value={`${sectionIssueCount}`}
                  tone={sectionIssueCount === 0 ? "emerald" : sectionIssueHasError ? "rose" : "sky"}
                />
              </div>
            </div>
          </section>

          <section className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6">
            <div className="flex flex-col gap-4 border-b border-white/10 pb-5 lg:flex-row lg:items-start lg:justify-between">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">detail</p>
                {navigationHistory.length > 0 && (
                  <button
                    type="button"
                    onClick={goBack}
                    className="mt-2 inline-flex items-center gap-1 rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs text-slate-300 transition hover:border-sky-400/40 hover:text-sky-200"
                  >
                    ← Back
                  </button>
                )}
                <h2 className="mt-2 text-2xl font-semibold text-white">
                  {currentItem
                    ? `${currentItem.id} — ${currentItem.title}`
                    : (currentDocument?.title ?? "No document selected")}
                </h2>
                {currentDocument ? (
                  <p className="mt-2 text-sm text-slate-400">{currentDocument.path}</p>
                ) : null}
              </div>
              <div className="flex flex-wrap gap-2">
                {currentItem?.status ? (
                  <MetaPill label="status" value={currentItem.status} />
                ) : null}
                {currentItem?.priority ? (
                  <MetaPill label="priority" value={currentItem.priority} />
                ) : null}
                {currentItem ? <MetaPill label="layer" value={currentItem.kind} /> : null}
              </div>
            </div>

            {currentDocument?.parse_error ? (
              <div className="mt-5 rounded-2xl border border-amber-400/30 bg-amber-400/10 px-4 py-4 text-sm text-amber-100">
                <p className="font-medium">
                  This document could not be parsed into the expected layer model.
                </p>
                <p className="mt-2 break-all font-mono text-xs text-amber-50/90">
                  File: {currentDocument.path}
                </p>
                <p className="mt-2 text-xs leading-6 text-amber-50/80">
                  {currentDocument.parse_error}
                </p>
              </div>
            ) : null}

            {currentDocument && currentDocument.items.length > 1 ? (
              <>
                <p className="mt-5 text-xs uppercase tracking-[0.25em] text-slate-500">
                  Items in this document
                </p>
                <div className="mt-2 flex flex-wrap gap-2">
                  {currentDocument.items.map((item) => {
                    const isActive = item.id === currentItem?.id;
                    return (
                      <button
                        key={item.id}
                        type="button"
                        title={item.title}
                        onClick={() => jumpToItem(item.id)}
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
              </>
            ) : null}

            {currentItem ? (
              <article className="mt-6 space-y-6">
                {infoPanels.length > 0 ? (
                  <div className="grid gap-4 xl:grid-cols-2">
                    {infoPanels.map((panel) => (
                      <InfoPanel key={panel.title} title={panel.title} content={panel.content} />
                    ))}
                  </div>
                ) : null}

                {relationshipPanels.length > 0 ? (
                  <div className="grid gap-4 xl:grid-cols-2">
                    {relationshipPanels.map((panel) => (
                      <RelationshipPanel
                        key={panel.label}
                        label={panel.label}
                        ids={panel.ids}
                        jumpToItem={jumpToItem}
                      />
                    ))}
                  </div>
                ) : null}

                {currentItem.tests.length > 0 ? (
                  <TracePanel label="Tests" groups={currentItem.tests} />
                ) : currentItem.status === "planned" && currentItem.kind === "requirement" ? (
                  <PlannedTracePlaceholder kind="tests" />
                ) : null}
                {currentItem.implementations.length > 0 ? (
                  <TracePanel label="Implementations" groups={currentItem.implementations} />
                ) : currentItem.status === "planned" && currentItem.kind === "feature" ? (
                  <PlannedTracePlaceholder kind="implementations" />
                ) : null}
              </article>
            ) : currentDocument ? (
              <div className="mt-6 rounded-2xl border border-dashed border-white/10 px-4 py-6 text-sm text-slate-400">
                This document is available as checked-in YAML, but it does not expose any parsed
                items for this layer.
              </div>
            ) : (
              <div className="mt-6 rounded-2xl border border-dashed border-white/10 px-4 py-6 text-sm text-slate-400">
                Choose a document from the left navigation to inspect its content.
              </div>
            )}
          </section>

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
              <div className="mt-5 grid gap-4 xl:grid-cols-[minmax(0,1fr)_minmax(0,20rem)]">
                <div className="space-y-3">
                  {workspace.validation.issues.map((issue, index) => {
                    const issueKey = validationIssueSelectionKey(issue);

                    return (
                      <button
                        key={`${issueKey}-${index}`}
                        type="button"
                        onClick={() => setSelectedIssueKey(issueKey)}
                        className={`w-full rounded-2xl border px-4 py-3 text-left transition ${
                          activeIssueIndex === index
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
                    );
                  })}
                </div>
                {activeIssue ? (
                  <IssueDetail
                    issue={activeIssue}
                    rule={activeRule}
                    workspace={workspace}
                    jumpToItem={jumpToItem}
                  />
                ) : null}
              </div>
            )}
          </section>

          <details
            defaultOpen={Boolean(currentDocument?.parse_error)}
            className="app-glass rounded-3xl border border-white/10 p-5 shadow-2xl shadow-sky-950/15 sm:p-6"
          >
            <summary className="flex cursor-pointer list-none items-center justify-between gap-3">
              <div>
                <p className="text-xs uppercase tracking-[0.3em] text-slate-500">source</p>
                <h2 className="mt-2 text-xl font-semibold text-white">Checked-in YAML</h2>
              </div>
              {currentDocument ? (
                <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
                  {currentDocument.path}
                </span>
              ) : null}
            </summary>
            <pre className="mt-5 overflow-x-auto rounded-2xl border border-white/10 bg-slate-950/80 p-4 text-sm leading-7 text-slate-200">
              {currentDocument?.raw_yaml ?? "No document selected."}
            </pre>
          </details>
        </section>
      </main>
    </div>
  );
}

function firstPopulatedSection(workspace: BrowserWorkspace): SectionKind | null {
  for (const kind of SECTION_ORDER) {
    const section = workspace.sections.find((candidate) => candidate.kind === kind);
    if (section && section.documents.length > 0) {
      return kind;
    }
  }

  return null;
}

function isSectionKind(value: string): value is SectionKind {
  return SECTION_ORDER.some((section) => section === value);
}

function validationIssueSelectionKey(issue: ValidationIssue): string {
  return [
    issue.code,
    issue.severity,
    issue.subject,
    issue.location ?? "",
    issue.message,
    issue.suggestion ?? "",
  ].join("\u0000");
}

function truncatePath(fullPath: string): string {
  const parts = fullPath.replace(/\\/g, "/").split("/").filter(Boolean);
  if (parts.length <= 2) return fullPath;
  return `…/${parts.slice(-2).join("/")}`;
}

function formatAppServerUrl(bind: string, port: number): string {
  const host = bind.includes(":") && !bind.startsWith("[") ? `[${bind}]` : bind;
  return `http://${host}:${port}`;
}

function errorMessage(error: unknown, fallback: string): string {
  return error instanceof Error ? error.message : fallback;
}

function formatRefreshFailure(action: string, error: unknown): string {
  return `Could not ${action}: ${errorMessage(error, "Unexpected refresh failure")}`;
}

async function describeAppDataRefreshFailure(response: Response): Promise<string> {
  const fallback = `Failed to load app data: ${response.status} ${response.statusText}`;
  const contentType = response.headers.get("content-type") ?? "";
  const body = await response.text();

  try {
    if (!contentType.includes("application/json") && !body.trim().startsWith("{")) {
      return fallback;
    }

    const payload = JSON.parse(body) as unknown;
    if (isAppDataErrorResponse(payload)) {
      return `${payload.error.summary} ${payload.error.guidance}`;
    }
  } catch {
    return fallback;
  }

  return fallback;
}

function isAppDataErrorResponse(value: unknown): value is AppDataErrorResponse {
  if (!value || typeof value !== "object") {
    return false;
  }

  const error = (value as { error?: unknown }).error;
  if (!error || typeof error !== "object") {
    return false;
  }

  const candidate = error as Partial<AppDataErrorResponse["error"]>;
  return (
    (candidate.code === "workspace-invalid" || candidate.code === "server-unavailable") &&
    typeof candidate.summary === "string" &&
    typeof candidate.guidance === "string"
  );
}

function formatRefreshTimestamp(iso: string | null): string {
  if (!iso) {
    return "Waiting for first successful load";
  }

  const timestamp = new Date(iso);
  if (Number.isNaN(timestamp.getTime())) {
    return iso;
  }

  return new Intl.DateTimeFormat(undefined, {
    dateStyle: "medium",
    timeStyle: "medium",
  }).format(timestamp);
}

function ratio(validated: number, declared: number): number {
  if (declared === 0) {
    return 0;
  }
  return Math.max(0, Math.min(1, validated / declared));
}

function onboardingStorageKey(workspaceRoot: string | null | undefined): string {
  const normalizedRoot = workspaceRoot?.trim();

  if (!normalizedRoot) {
    return ONBOARDING_STORAGE_KEY;
  }

  return `${ONBOARDING_STORAGE_KEY}:${normalizedRoot}`;
}

function shouldShowOnboarding(workspaceRoot?: string): boolean {
  if (typeof window === "undefined") {
    return true;
  }

  try {
    return window.sessionStorage.getItem(onboardingStorageKey(workspaceRoot)) !== "true";
  } catch (error) {
    console.warn("syu app could not read onboarding dismissal state from sessionStorage.", error);
    return true;
  }
}

function persistOnboardingDismissal(workspaceRoot?: string) {
  if (typeof window === "undefined") {
    return;
  }

  try {
    window.sessionStorage.setItem(onboardingStorageKey(workspaceRoot), "true");
  } catch (error) {
    console.warn("syu app could not persist onboarding dismissal in sessionStorage.", error);
  }
}

function formatTraceSymbols(symbols: string[]): string {
  const normalized = symbols.map((symbol) => symbol.trim()).filter((symbol) => symbol.length > 0);

  if (normalized.length === 0) {
    return "none listed";
  }

  if (normalized.some((symbol) => symbol === "*")) {
    return "any symbol (wildcard)";
  }

  return normalized.join(", ");
}

function InfoHint({ label, description }: { label: string; description: string }) {
  return (
    <button
      type="button"
      aria-label={`${label}: ${description}`}
      title={description}
      className="inline-flex h-5 w-5 items-center justify-center rounded-full border border-white/10 bg-white/5 align-middle text-[10px] normal-case leading-none tracking-normal text-slate-400 transition hover:border-sky-400/40 hover:text-sky-200 focus:outline-none focus-visible:border-sky-400/60 focus-visible:ring-2 focus-visible:ring-sky-400/30"
    >
      ⓘ
    </button>
  );
}

function LayerNavigationCard({
  summary,
  active,
  maxItems,
  onSelect,
}: {
  summary: SectionSummary;
  active: boolean;
  maxItems: number;
  onSelect: () => void;
}) {
  const barWidth =
    summary.itemCount === 0 ? 14 : Math.max(18, (summary.itemCount / maxItems) * 100);

  return (
    <button
      type="button"
      onClick={onSelect}
      className={`w-full rounded-3xl border px-4 py-4 text-left transition ${
        active
          ? "border-sky-400/70 bg-sky-400/12 text-sky-50 shadow-lg shadow-sky-950/20"
          : "border-white/10 bg-slate-950/60 text-slate-300 hover:border-sky-400/40 hover:text-white"
      }`}
    >
      <div className="flex items-start justify-between gap-4">
        <div>
          <p className="text-base font-semibold capitalize text-white">{summary.label}</p>
          <p className="mt-1 text-sm leading-6 text-slate-400">{SECTION_COPY[summary.kind]}</p>
        </div>
        <span className="rounded-full border border-white/10 bg-white/5 px-3 py-1 text-xs uppercase tracking-[0.2em] text-slate-400">
          {summary.documentCount === 1 ? "1 doc" : `${summary.documentCount} docs`}
        </span>
      </div>
      <div className="mt-4 flex items-center justify-between text-[11px] uppercase tracking-[0.2em] text-slate-500">
        <span>{summary.itemCount} items</span>
        <span>{summary.documentCount === 1 ? "single document" : "grouped navigation"}</span>
      </div>
      <div
        className="mt-3 h-2 rounded-full bg-white/5"
        role="progressbar"
        aria-label={`${summary.label} item count`}
        aria-valuenow={summary.itemCount}
        aria-valuemin={0}
        aria-valuemax={maxItems}
        aria-valuetext={`${summary.itemCount} of ${maxItems} items`}
      >
        <div
          className={`h-full rounded-full ${active ? "bg-sky-300" : "bg-slate-400/70"}`}
          style={{ width: `${barWidth}%` }}
        />
      </div>
    </button>
  );
}

function SummaryStat({
  label,
  value,
  tone = "sky",
}: {
  label: string;
  value: string;
  tone?: "sky" | "rose" | "emerald";
}) {
  const toneClass =
    tone === "rose"
      ? "border-rose-400/30 bg-rose-400/10 text-rose-100"
      : tone === "emerald"
        ? "border-emerald-400/30 bg-emerald-400/10 text-emerald-100"
        : "border-white/10 bg-white/5 text-slate-100";

  return (
    <div className={`rounded-2xl border px-4 py-3 ${toneClass}`}>
      <p className="text-[11px] uppercase tracking-[0.25em] text-slate-400">{label}</p>
      <p className="mt-2 text-2xl font-semibold">{value}</p>
    </div>
  );
}

function CompactMetric({
  label,
  value,
  note,
  tone = "sky",
  ratio,
  hint,
}: {
  label: string;
  value: string;
  note: string;
  tone?: "sky" | "violet";
  ratio?: number;
  hint?: {
    label: string;
    description: string;
  };
}) {
  const barClass = tone === "violet" ? "bg-violet-300" : "bg-sky-300";

  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/60 px-4 py-3">
      <p className="text-[11px] uppercase tracking-[0.25em] text-slate-500">{label}</p>
      <p className="mt-2 text-lg font-semibold text-white">{value}</p>
      <div className="mt-1 flex items-center gap-1.5 text-xs text-slate-400">
        <p>{note}</p>
        {hint ? <InfoHint label={hint.label} description={hint.description} /> : null}
      </div>
      {typeof ratio === "number" ? (
        <div className="mt-3 h-2 rounded-full bg-white/5">
          <div className={`h-full rounded-full ${barClass}`} style={{ width: `${ratio * 100}%` }} />
        </div>
      ) : null}
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

function InfoPanel({ title, content }: { title: string; content: string }) {
  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/50 p-4">
      <p className="text-xs uppercase tracking-[0.25em] text-slate-500">{title}</p>
      <p className="mt-3 text-sm leading-7 text-slate-200">{content}</p>
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
    </div>
  );
}

function PlannedTracePlaceholder({ kind }: { kind: "tests" | "implementations" }) {
  const label = kind === "tests" ? "Tests" : "Implementations";
  const field = kind === "tests" ? "tests" : "implementations";
  const exampleId = kind === "tests" ? "REQ-MY-001" : "FEAT-MY-001";
  const exampleSymbol = kind === "tests" ? "my_test_function" : "my_impl_function";
  const yamlExample = `${field}:\n  rust:\n    - file: src/my_file.rs\n      symbols:\n        - ${exampleSymbol}\n      doc_contains:\n        - ${exampleId}`;

  return (
    <div className="rounded-2xl border border-dashed border-sky-400/20 bg-sky-950/20 p-4">
      <div className="flex items-center gap-2">
        <p className="text-xs uppercase tracking-[0.25em] text-sky-400/60">{label}</p>
        <span className="rounded-full border border-sky-400/20 bg-sky-400/10 px-2 py-0.5 text-[10px] text-sky-300/70">
          not yet declared
        </span>
      </div>
      <p className="mt-3 text-sm text-slate-400">
        This item is <span className="text-sky-300">planned</span>. Add a{" "}
        <code className="rounded bg-white/5 px-1 py-0.5 font-mono text-xs text-slate-200">
          {field}:
        </code>{" "}
        block when you implement it:
      </p>
      <pre className="mt-3 overflow-x-auto rounded-xl border border-white/5 bg-slate-950/70 p-3 text-xs leading-6 text-slate-300">
        {yamlExample}
      </pre>
      <p className="mt-3 text-xs text-slate-500">
        Then change <code className="rounded bg-white/5 px-1 font-mono text-slate-400">status</code>{" "}
        to <code className="rounded bg-white/5 px-1 font-mono text-slate-400">implemented</code> and
        run <code className="rounded bg-white/5 px-1 font-mono text-slate-400">syu validate .</code>{" "}
        to verify the traces.
      </p>
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
                    symbols{" "}
                    <InfoHint
                      label="Symbols"
                      description="The function, struct, method, or constant names that this trace points to. Use * to match the whole file."
                    />
                  </p>
                  <p className="mt-1 text-sm text-slate-300">
                    {formatTraceSymbols(reference.symbols)}
                  </p>
                  <p className="mt-3 text-xs uppercase tracking-[0.2em] text-slate-500">
                    doc contains{" "}
                    <InfoHint
                      label="Doc contains"
                      description="A string that must appear in the symbol's documentation comment. 'not declared' means no assertion is declared."
                    />
                  </p>
                  <p className="mt-1 text-sm text-slate-300">
                    {reference.doc_contains.length > 0
                      ? reference.doc_contains.join(", ")
                      : "not declared"}
                  </p>
                </div>
              ))}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function IssueDetail({
  issue,
  rule,
  workspace,
  jumpToItem,
}: {
  issue: ValidationIssue;
  rule: ReferencedRule | null;
  workspace: BrowserWorkspace | null;
  jumpToItem: (id: string) => void;
}) {
  const subjectInIndex = workspace?.item_index.get(issue.subject) != null;

  return (
    <div className="rounded-2xl border border-white/10 bg-slate-950/70 p-4">
      <p className="text-xs uppercase tracking-[0.25em] text-slate-500">selected issue</p>
      <h3 className="mt-2 text-lg font-semibold text-white">{issue.code}</h3>
      <p className="mt-3 text-sm leading-7 text-slate-200">{issue.message}</p>
      {issue.location ? (
        <p className="mt-3 text-xs uppercase tracking-[0.2em] text-slate-500">
          location:{" "}
          <span className="normal-case tracking-normal text-slate-300">{issue.location}</span>
        </p>
      ) : null}
      {subjectInIndex ? (
        <button
          type="button"
          onClick={() => jumpToItem(issue.subject)}
          className="mt-4 flex items-center gap-1.5 rounded-full border border-sky-400/30 bg-sky-400/10 px-3 py-1.5 text-sm text-sky-300 transition hover:border-sky-400/60 hover:bg-sky-400/20"
        >
          <span>→</span>
          <span>
            View <span className="font-mono">{issue.subject}</span>
          </span>
        </button>
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
