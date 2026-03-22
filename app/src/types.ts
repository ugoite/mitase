export type SectionKind = "philosophy" | "policies" | "features" | "requirements";

export interface SourceDocument {
  section: SectionKind;
  path: string;
  content: string;
}

export interface DefinitionCounts {
  philosophies: number;
  policies: number;
  requirements: number;
  features: number;
}

export interface TraceCount {
  declared: number;
  validated: number;
}

export interface TraceSummary {
  requirement_traces: TraceCount;
  feature_traces: TraceCount;
}

export interface ValidationIssue {
  code: string;
  severity: "error" | "warning";
  subject: string;
  location: string | null;
  message: string;
  suggestion: string | null;
}

export interface ReferencedRule {
  genre: string;
  code: string;
  severity: string;
  title: string;
  summary: string;
  description: string;
}

export interface ValidationSnapshot {
  definition_counts: DefinitionCounts;
  trace_summary: TraceSummary;
  issues: ValidationIssue[];
  referenced_rules: ReferencedRule[];
}

export interface AppPayload {
  workspace_root: string;
  spec_root: string;
  source_documents: SourceDocument[];
  validation: ValidationSnapshot;
}

export interface BrowserTraceReference {
  file: string;
  symbols: string[];
  doc_contains: string[];
}

export interface BrowserTraceGroup {
  language: string;
  references: BrowserTraceReference[];
}

export interface BrowserItem {
  kind: SectionKind;
  id: string;
  title: string;
  summary: string | null;
  description: string | null;
  product_design_principle: string | null;
  coding_guideline: string | null;
  priority: string | null;
  status: string | null;
  linked_philosophies: string[];
  linked_policies: string[];
  linked_requirements: string[];
  linked_features: string[];
  tests: BrowserTraceGroup[];
  implementations: BrowserTraceGroup[];
}

export interface BrowserDocument {
  section: SectionKind;
  path: string;
  title: string;
  folder_segments: string[];
  raw_yaml: string;
  parse_error: string | null;
  items: BrowserItem[];
}

export interface BrowserSection {
  kind: SectionKind;
  label: string;
  documents: BrowserDocument[];
}

export interface BrowserIndexEntry {
  id: string;
  title: string;
  kind: SectionKind;
  document_path: string;
}

export interface BrowserWorkspace {
  workspace_root: string;
  spec_root: string;
  sections: BrowserSection[];
  item_index: Map<string, BrowserIndexEntry>;
  validation: ValidationSnapshot;
}
