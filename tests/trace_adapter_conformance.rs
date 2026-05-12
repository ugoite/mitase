// REQ-CORE-002
// REQ-CORE-003

use std::fs;
use syu::{
    config::SyuConfig,
    coverage::supports_strict_inventory,
    inspect::{apply_symbol_doc_fix, inspect_symbol, supports_rich_inspection},
    language::{LanguageAdapter, adapter_for_language},
};
use tempfile::tempdir;

struct TraceAdapterCase {
    label: &'static str,
    row_label: &'static str,
    language: &'static str,
    canonical_name: &'static str,
    path: &'static str,
    source: &'static str,
    symbol: &'static str,
    doc_contains: bool,
    strict_inventory: bool,
}

const CAPABILITY_SNIPPET: &str = "capability harness";

const CASES: &[TraceAdapterCase] = &[
    TraceAdapterCase {
        label: "Rust",
        row_label: "Rust",
        language: "rust",
        canonical_name: "rust",
        path: "src/trace.rs",
        source: "/// existing docs\npub fn trace_harness_rust() {}\n",
        symbol: "trace_harness_rust",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "Python",
        row_label: "Python",
        language: "python",
        canonical_name: "python",
        path: "src/trace.py",
        source: "def trace_harness_python():\n    \"\"\"existing docs\"\"\"\n    pass\n",
        symbol: "trace_harness_python",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "Ruby",
        row_label: "Ruby",
        language: "ruby",
        canonical_name: "ruby",
        path: "src/trace.rb",
        source: "# existing docs\ndef trace_harness_ruby\n  true\nend\n",
        symbol: "trace_harness_ruby",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "Go",
        row_label: "Go",
        language: "go",
        canonical_name: "go",
        path: "src/trace.go",
        source: "// existing docs\nfunc TraceHarnessGo() {}\n",
        symbol: "TraceHarnessGo",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "Java",
        row_label: "Java",
        language: "java",
        canonical_name: "java",
        path: "src/TraceHarnessJava.java",
        source: "/** existing docs */\npublic class TraceHarnessJava {}\n",
        symbol: "TraceHarnessJava",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "C#",
        row_label: "C#",
        language: "csharp",
        canonical_name: "csharp",
        path: "src/TraceHarnessCSharp.cs",
        source: "/// existing docs\npublic class TraceHarnessCSharp {}\n",
        symbol: "TraceHarnessCSharp",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "Kotlin",
        row_label: "Kotlin",
        language: "kotlin",
        canonical_name: "kotlin",
        path: "src/TraceHarnessKotlin.kt",
        source: "/** existing docs */\nfun traceHarnessKotlin() {}\n",
        symbol: "traceHarnessKotlin",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "TypeScript",
        row_label: "TypeScript / JavaScript",
        language: "typescript",
        canonical_name: "typescript",
        path: "src/trace-harness.ts",
        source: "/** existing docs */\nexport function traceHarnessTs() {}\n",
        symbol: "traceHarnessTs",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "JavaScript",
        row_label: "TypeScript / JavaScript",
        language: "javascript",
        canonical_name: "typescript",
        path: "src/trace-harness.js",
        source: "/** existing docs */\nexport function traceHarnessJs() {}\n",
        symbol: "traceHarnessJs",
        doc_contains: true,
        strict_inventory: true,
    },
    TraceAdapterCase {
        label: "Shell",
        row_label: "Shell",
        language: "shell",
        canonical_name: "shell",
        path: "scripts/trace.sh",
        source: "trace_harness_shell() { :; }\n",
        symbol: "trace_harness_shell",
        doc_contains: false,
        strict_inventory: false,
    },
    TraceAdapterCase {
        label: "YAML",
        row_label: "YAML",
        language: "yaml",
        canonical_name: "yaml",
        path: "config/trace.yaml",
        source: "trace_harness_yaml: true\n",
        symbol: "trace_harness_yaml",
        doc_contains: false,
        strict_inventory: false,
    },
    TraceAdapterCase {
        label: "JSON",
        row_label: "JSON",
        language: "json",
        canonical_name: "json",
        path: "config/trace.json",
        source: "{\"trace_harness_json\": true}\n",
        symbol: "trace_harness_json",
        doc_contains: false,
        strict_inventory: false,
    },
    TraceAdapterCase {
        label: "Markdown",
        row_label: "Markdown",
        language: "markdown",
        canonical_name: "markdown",
        path: "README.md",
        source: "# trace_harness_markdown\n",
        symbol: "trace_harness_markdown",
        doc_contains: false,
        strict_inventory: false,
    },
    TraceAdapterCase {
        label: "Gitignore",
        row_label: "Gitignore",
        language: "gitignore",
        canonical_name: "gitignore",
        path: ".gitignore",
        source: "trace_harness_gitignore\n",
        symbol: "trace_harness_gitignore",
        doc_contains: false,
        strict_inventory: false,
    },
];

fn checkmark(value: bool) -> &'static str {
    if value { "✅" } else { "❌" }
}

fn backticked_list(values: &[&str]) -> String {
    values
        .iter()
        .map(|value| format!("`{value}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn display_extensions(adapter: &dyn LanguageAdapter) -> String {
    adapter
        .extensions()
        .iter()
        .map(|extension| format!("`.{extension}`"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn expected_row(case: &TraceAdapterCase, adapter: &dyn LanguageAdapter) -> String {
    format!(
        "| {} | {} / {} | {} | {} | {} | {} |",
        case.row_label,
        backticked_list(adapter.aliases()),
        display_extensions(adapter),
        checkmark(true),
        checkmark(case.doc_contains),
        checkmark(case.strict_inventory),
        checkmark(case.doc_contains),
    )
}

#[test]
fn built_in_trace_adapters_share_one_capability_matrix() {
    let documentation = std::fs::read_to_string("docs/guide/trace-adapter-support.md")
        .expect("trace adapter support guide should exist");

    for case in CASES {
        let adapter = adapter_for_language(case.language).expect("adapter should exist");
        assert_eq!(
            adapter.canonical_name(),
            case.canonical_name,
            "unexpected canonical adapter for {}",
            case.label
        );
        assert!(adapter.symbol_exists(case.source, case.symbol));
        assert_eq!(
            supports_rich_inspection(case.language),
            case.doc_contains,
            "doc_contains capability drift for {}",
            case.label
        );
        assert_eq!(
            supports_strict_inventory(case.language),
            case.strict_inventory,
            "strict inventory capability drift for {}",
            case.label
        );

        if case.doc_contains {
            let tempdir = tempdir().expect("tempdir should exist");
            let path = tempdir.path().join(case.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent directory should exist");
            }
            fs::write(&path, case.source).expect("sample file should be writable");

            let inspected = inspect_symbol(
                case.language,
                &SyuConfig::default(),
                &path,
                case.source,
                case.symbol,
            )
            .expect("inspection should succeed")
            .expect("rich inspection should find the symbol");

            assert!(
                inspected.docs.contains("existing docs"),
                "expected doc inspection for {}",
                case.label
            );

            let updated = apply_symbol_doc_fix(
                case.language,
                &SyuConfig::default(),
                &path,
                case.source,
                case.symbol,
                &[CAPABILITY_SNIPPET.to_string()],
            )
            .expect("doc autofix should succeed")
            .expect("rich inspection languages should support autofix");

            assert!(
                updated.contains(CAPABILITY_SNIPPET),
                "expected autofix for {} to insert the requested snippet",
                case.label
            );
        } else {
            let tempdir = tempdir().expect("tempdir should exist");
            let path = tempdir.path().join(case.path);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).expect("parent directory should exist");
            }
            fs::write(&path, case.source).expect("sample file should be writable");

            assert_eq!(
                inspect_symbol(
                    case.language,
                    &SyuConfig::default(),
                    &path,
                    case.source,
                    case.symbol,
                )
                .expect("inspection should not fail"),
                None
            );
        }

        let expected_row = expected_row(case, adapter);
        assert!(
            documentation.contains(&expected_row),
            "missing matrix row for {}: {expected_row}",
            case.label
        );
    }
}
