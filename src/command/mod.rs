use std::path::Path;

// FEAT-BROWSE-001

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
        return "''".to_string();
    }

    if rendered
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || "/._-".contains(ch))
    {
        rendered
    } else {
        format!("'{}'", rendered.replace('\'', "'\\''"))
    }
}

#[cfg(test)]
// REQ-CORE-009
mod tests {
    use std::path::Path;

    use super::shell_quote_path;

    #[test]
    fn shell_quote_path_wraps_empty_paths() {
        assert_eq!(shell_quote_path(Path::new("")), "''");
    }

    #[test]
    fn shell_quote_path_escapes_special_characters() {
        assert_eq!(
            shell_quote_path(Path::new("workspace with 'quotes'")),
            "'workspace with '\\''quotes'\\'''"
        );
    }
}
