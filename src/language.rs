// FEAT-CHECK-001
// REQ-CORE-002

use regex::Regex;
use std::path::Path;

pub trait LanguageAdapter: Sync {
    fn canonical_name(&self) -> &'static str;
    fn aliases(&self) -> &'static [&'static str];
    fn extensions(&self) -> &'static [&'static str];
    fn patterns(&self, symbol: &str) -> Vec<String>;

    fn supports_path(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|ext| ext.to_str())
            .map(|ext| {
                self.extensions()
                    .iter()
                    .any(|candidate| candidate.eq_ignore_ascii_case(ext))
            })
            .unwrap_or(false)
    }

    fn symbol_exists(&self, contents: &str, symbol: &str) -> bool {
        if !is_identifier(symbol) {
            return contents.contains(symbol);
        }

        self.patterns(symbol)
            .into_iter()
            .filter_map(|pattern| Regex::new(&pattern).ok())
            .any(|regex| regex.is_match(contents))
    }
}

#[derive(Debug)]
struct RustAdapter;

#[derive(Debug)]
struct PythonAdapter;

#[derive(Debug)]
struct TypeScriptAdapter;

#[derive(Debug)]
struct GoAdapter;

#[derive(Debug)]
struct JavaAdapter;

#[derive(Debug)]
struct ShellAdapter;

#[derive(Debug)]
struct YamlAdapter;

#[derive(Debug)]
struct JsonAdapter;

#[derive(Debug)]
struct MarkdownAdapter;

#[derive(Debug)]
struct GitignoreAdapter;

static RUST_ADAPTER: RustAdapter = RustAdapter;
static PYTHON_ADAPTER: PythonAdapter = PythonAdapter;
static TYPESCRIPT_ADAPTER: TypeScriptAdapter = TypeScriptAdapter;
static GO_ADAPTER: GoAdapter = GoAdapter;
static JAVA_ADAPTER: JavaAdapter = JavaAdapter;
static SHELL_ADAPTER: ShellAdapter = ShellAdapter;
static YAML_ADAPTER: YamlAdapter = YamlAdapter;
static JSON_ADAPTER: JsonAdapter = JsonAdapter;
static MARKDOWN_ADAPTER: MarkdownAdapter = MarkdownAdapter;
static GITIGNORE_ADAPTER: GitignoreAdapter = GitignoreAdapter;

impl LanguageAdapter for RustAdapter {
    fn canonical_name(&self) -> &'static str {
        "rust"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["rust", "rs"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["rs"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![
            format!(r"(?m)\b(?:pub(?:\([^)]*\))?\s+)?fn\s+{escaped}\b"),
            format!(r"(?m)\b(?:pub(?:\([^)]*\))?\s+)?(?:struct|enum|trait)\s+{escaped}\b"),
            format!(r"(?m)\b(?:pub(?:\([^)]*\))?\s+)?(?:const|static)\s+{escaped}\b"),
            format!(r"(?m)\b{escaped}\b"),
        ]
    }
}

impl LanguageAdapter for PythonAdapter {
    fn canonical_name(&self) -> &'static str {
        "python"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["python", "py", "pytest", "unittest"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["py"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![
            format!(r"(?m)\bdef\s+{escaped}\b"),
            format!(r"(?m)\bclass\s+{escaped}\b"),
            format!(r"(?m)^\s*{escaped}\s*="),
            format!(r"(?m)\b{escaped}\b"),
        ]
    }
}

impl LanguageAdapter for TypeScriptAdapter {
    fn canonical_name(&self) -> &'static str {
        "typescript"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &[
            "typescript",
            "ts",
            "tsx",
            "javascript",
            "js",
            "jsx",
            "vitest",
            "bun",
            "bun-test",
        ]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["ts", "tsx", "js", "jsx"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![
            format!(r"(?m)\b(?:export\s+)?(?:default\s+)?(?:async\s+)?function\s+{escaped}\b"),
            format!(r"(?m)\b(?:export\s+)?(?:const|let|var|class|interface|type)\s+{escaped}\b"),
            format!(r"(?m)\b{escaped}\b"),
        ]
    }
}

impl LanguageAdapter for GoAdapter {
    fn canonical_name(&self) -> &'static str {
        "go"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["go", "golang", "gotest"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["go"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![
            format!(r"(?m)\bfunc\s+(?:\([^)]+\)\s*)?{escaped}\b"),
            format!(r"(?m)\btype\s+{escaped}\b"),
            format!(r"(?m)\b(?:var|const)\s+{escaped}\b"),
            format!(r"(?ms)\b(?:var|const)\s*\([^)]*\b{escaped}\b"),
            format!(r"(?m)\b{escaped}\b"),
        ]
    }
}

impl LanguageAdapter for JavaAdapter {
    fn canonical_name(&self) -> &'static str {
        "java"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["java", "junit"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["java"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![
            format!(r"(?m)\b(?:class|interface|enum|record)\s+{escaped}\b"),
            format!(r"(?m)\b{escaped}\s*\("),
            format!(r"(?m)\b{escaped}\s*(?:=|;)"),
            format!(r"(?m)\b{escaped}\b"),
        ]
    }
}

impl LanguageAdapter for ShellAdapter {
    fn canonical_name(&self) -> &'static str {
        "shell"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["shell", "sh", "bash", "zsh"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["sh", "bash", "zsh"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![
            format!(r"(?m)^\s*(?:function\s+)?{escaped}\s*\(\)"),
            format!(r"(?m)\b{escaped}\b"),
        ]
    }
}

impl LanguageAdapter for YamlAdapter {
    fn canonical_name(&self) -> &'static str {
        "yaml"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["yaml", "yml"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![format!(r"(?m)\b{escaped}\b")]
    }
}

impl LanguageAdapter for JsonAdapter {
    fn canonical_name(&self) -> &'static str {
        "json"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["json"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["json"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![format!(r#"(?m)"{escaped}""#), format!(r"(?m)\b{escaped}\b")]
    }
}

impl LanguageAdapter for MarkdownAdapter {
    fn canonical_name(&self) -> &'static str {
        "markdown"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["markdown", "md"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["md"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![format!(r"(?m)\b{escaped}\b")]
    }
}

impl LanguageAdapter for GitignoreAdapter {
    fn canonical_name(&self) -> &'static str {
        "gitignore"
    }

    fn aliases(&self) -> &'static [&'static str] {
        &["gitignore", "ignore"]
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["gitignore"]
    }

    fn patterns(&self, symbol: &str) -> Vec<String> {
        let escaped = regex::escape(symbol);
        vec![format!(r"(?m)\b{escaped}\b")]
    }

    fn supports_path(&self, path: &Path) -> bool {
        path.file_name()
            .and_then(|name| name.to_str())
            .map(|name| name.eq_ignore_ascii_case(".gitignore"))
            .unwrap_or(false)
    }
}

// FEAT-CHECK-001
pub fn adapter_for_language(language: &str) -> Option<&'static dyn LanguageAdapter> {
    let normalized = language.trim().to_ascii_lowercase().replace('_', "-");

    [
        &RUST_ADAPTER as &dyn LanguageAdapter,
        &PYTHON_ADAPTER as &dyn LanguageAdapter,
        &TYPESCRIPT_ADAPTER as &dyn LanguageAdapter,
        &GO_ADAPTER as &dyn LanguageAdapter,
        &JAVA_ADAPTER as &dyn LanguageAdapter,
        &SHELL_ADAPTER as &dyn LanguageAdapter,
        &YAML_ADAPTER as &dyn LanguageAdapter,
        &JSON_ADAPTER as &dyn LanguageAdapter,
        &MARKDOWN_ADAPTER as &dyn LanguageAdapter,
        &GITIGNORE_ADAPTER as &dyn LanguageAdapter,
    ]
    .into_iter()
    .find(|adapter| adapter.aliases().iter().any(|alias| *alias == normalized))
}

fn is_identifier(symbol: &str) -> bool {
    !symbol.is_empty()
        && symbol
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '_')
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{LanguageAdapter, adapter_for_language};

    #[test]
    fn adapter_lookup_supports_all_known_aliases() {
        assert_eq!(
            adapter_for_language("rust").map(LanguageAdapter::canonical_name),
            Some("rust")
        );
        assert_eq!(
            adapter_for_language("pytest").map(LanguageAdapter::canonical_name),
            Some("python")
        );
        assert_eq!(
            adapter_for_language("bun-test").map(LanguageAdapter::canonical_name),
            Some("typescript")
        );
        assert_eq!(
            adapter_for_language("bash").map(LanguageAdapter::canonical_name),
            Some("shell")
        );
        assert_eq!(
            adapter_for_language("golang").map(LanguageAdapter::canonical_name),
            Some("go")
        );
        assert_eq!(
            adapter_for_language("yml").map(LanguageAdapter::canonical_name),
            Some("yaml")
        );
        assert_eq!(
            adapter_for_language("json").map(LanguageAdapter::canonical_name),
            Some("json")
        );
        assert_eq!(
            adapter_for_language("md").map(LanguageAdapter::canonical_name),
            Some("markdown")
        );
        assert_eq!(
            adapter_for_language("ignore").map(LanguageAdapter::canonical_name),
            Some("gitignore")
        );
        assert!(adapter_for_language("unknown").is_none());
    }

    #[test]
    fn adapters_match_supported_extensions() {
        let rust = adapter_for_language("rust").expect("rust adapter should exist");
        let go = adapter_for_language("go").expect("go adapter should exist");
        let shell = adapter_for_language("shell").expect("shell adapter should exist");
        let yaml = adapter_for_language("yaml").expect("yaml adapter should exist");
        let json = adapter_for_language("json").expect("json adapter should exist");
        let markdown = adapter_for_language("markdown").expect("markdown adapter should exist");
        let gitignore = adapter_for_language("gitignore").expect("gitignore adapter should exist");

        assert!(rust.supports_path(Path::new("src/lib.rs")));
        assert!(go.supports_path(Path::new("go/app.go")));
        assert!(!rust.supports_path(Path::new("src/lib.py")));
        assert!(shell.supports_path(Path::new("scripts/install-syu.sh")));
        assert!(yaml.supports_path(Path::new(".github/workflows/ci.yml")));
        assert!(json.supports_path(Path::new("release-please-config.json")));
        assert!(markdown.supports_path(Path::new("README.md")));
        assert!(gitignore.supports_path(Path::new(".gitignore")));
        assert!(gitignore.supports_path(Path::new("app/.gitignore")));
        assert_eq!(gitignore.extensions(), &["gitignore"]);
        assert!(!yaml.supports_path(Path::new("README.md")));
        assert!(!gitignore.supports_path(Path::new("README.md")));
    }

    #[test]
    fn adapters_find_symbols_in_source_text() {
        let rust = adapter_for_language("rust").expect("rust adapter should exist");
        let python = adapter_for_language("python").expect("python adapter should exist");
        let typescript =
            adapter_for_language("typescript").expect("typescript adapter should exist");
        let go = adapter_for_language("go").expect("go adapter should exist");
        let shell = adapter_for_language("shell").expect("shell adapter should exist");
        let yaml = adapter_for_language("yaml").expect("yaml adapter should exist");
        let json = adapter_for_language("json").expect("json adapter should exist");
        let markdown = adapter_for_language("markdown").expect("markdown adapter should exist");
        let gitignore = adapter_for_language("gitignore").expect("gitignore adapter should exist");

        assert!(rust.symbol_exists("pub fn hello_world() {}", "hello_world"));
        assert!(python.symbol_exists("def test_trace():\n    return True\n", "test_trace"));
        assert!(typescript.symbol_exists(
            "export function featureTraceTs(): boolean { return true; }",
            "featureTraceTs"
        ));
        assert!(go.symbol_exists(
            "func GoFeatureImpl() string { return \"ok\" }\n",
            "GoFeatureImpl"
        ));
        assert!(go.symbol_exists("func GenericAPI[T any]() {}\n", "GenericAPI"));
        assert!(go.symbol_exists(
            "func (svc Service) TestGoRequirement(t *testing.T) {}\n",
            "TestGoRequirement"
        ));
        assert!(go.symbol_exists("const (\n    ExportedFlag = true\n)\n", "ExportedFlag"));
        assert!(shell.symbol_exists("install_syu() {\n  echo ok\n}\n", "install_syu"));
        assert!(yaml.symbol_exists("name: CI\njobs:\n  quality:\n", "quality"));
        assert!(json.symbol_exists("{\"package\":\"syu\"}", "package"));
        assert!(json.symbol_exists("{\"package-name\":\"syu\"}", "package-name"));
        assert!(markdown.symbol_exists("# syu\n\nSee `docs/guide/concepts.md`\n", "concepts"));
        assert!(gitignore.symbol_exists("# FEAT-CONTRIB-002\n/.worktrees/\n", "FEAT-CONTRIB-002"));
        assert!(gitignore.symbol_exists("# FEAT-CONTRIB-002\n/.worktrees/\n", "/.worktrees/"));
        assert!(gitignore.symbol_exists("worktree_helper\n", "worktree_helper"));
    }

    #[test]
    fn symbol_exists_handles_non_identifier_literals() {
        let yaml = adapter_for_language("yaml").expect("yaml adapter should exist");
        assert!(yaml.symbol_exists(
            "uses: googleapis/release-please-action@v4",
            "googleapis/release-please-action@v4"
        ));
        assert!(!yaml.symbol_exists(
            "uses: rhysd/actionlint@v1",
            "googleapis/release-please-action@v4"
        ));
    }
}
