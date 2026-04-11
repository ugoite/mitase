use std::path::Path;

// FEAT-BROWSE-001

pub mod add;
pub mod app;
pub mod browse;
pub mod check;
pub mod init;
pub mod list;
mod lookup;
pub mod report;
pub mod show;

pub(crate) fn shell_quote_path(path: &Path) -> String {
    let rendered = path.display().to_string();
    if rendered.is_empty() {
        return empty_shell_path();
    }

    if is_shell_safe_path(&rendered) {
        rendered
    } else {
        quote_shell_path(&rendered)
    }
}

#[cfg(windows)]
fn empty_shell_path() -> String {
    "\"\"".to_string()
}

#[cfg(not(windows))]
fn empty_shell_path() -> String {
    "''".to_string()
}

#[cfg(windows)]
fn quote_shell_path(rendered: &str) -> String {
    format!("\"{rendered}\"")
}

#[cfg(not(windows))]
fn quote_shell_path(rendered: &str) -> String {
    format!("'{}'", rendered.replace('\'', "'\\''"))
}

#[cfg(windows)]
fn is_shell_safe_path(rendered: &str) -> bool {
    rendered
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "/\\\\:._-".contains(ch))
}

#[cfg(not(windows))]
fn is_shell_safe_path(rendered: &str) -> bool {
    rendered
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "/._-".contains(ch))
}
#[cfg(test)]
// REQ-CORE-009
mod tests {
    use std::path::Path;

    use super::shell_quote_path;

    #[test]
    fn shell_quote_path_wraps_empty_paths() {
        assert_eq!(shell_quote_path(Path::new("")), expected_empty_shell_path());
    }

    #[test]
    fn shell_quote_path_escapes_special_characters() {
        assert_eq!(
            shell_quote_path(Path::new("workspace with 'quotes'")),
            expected_quoted_path()
        );
    }

    #[cfg(windows)]
    fn expected_empty_shell_path() -> &'static str {
        "\"\""
    }

    #[cfg(not(windows))]
    fn expected_empty_shell_path() -> &'static str {
        "''"
    }

    #[cfg(windows)]
    fn expected_quoted_path() -> &'static str {
        "\"workspace with 'quotes'\""
    }

    #[cfg(not(windows))]
    fn expected_quoted_path() -> &'static str {
        "'workspace with '\\''quotes'\\'''"
    }
}
