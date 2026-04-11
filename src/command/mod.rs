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
        return if cfg!(windows) {
            "\"\"".to_string()
        } else {
            "''".to_string()
        };
    }

    if is_shell_safe_path(&rendered) {
        rendered
    } else if cfg!(windows) {
        format!("\"{rendered}\"")
    } else {
        format!("'{}'", rendered.replace('\'', "'\\''"))
    }
}

fn is_shell_safe_path(rendered: &str) -> bool {
    rendered.chars().all(|ch| {
        ch.is_ascii_alphanumeric()
            || if cfg!(windows) {
                "/\\\\:._-".contains(ch)
            } else {
                "/._-".contains(ch)
            }
    })
}
#[cfg(test)]
// REQ-CORE-009
mod tests {
    use std::path::Path;

    use super::shell_quote_path;

    #[test]
    fn shell_quote_path_wraps_empty_paths() {
        let expected = if cfg!(windows) { "\"\"" } else { "''" };
        assert_eq!(shell_quote_path(Path::new("")), expected);
    }

    #[test]
    fn shell_quote_path_escapes_special_characters() {
        let expected = if cfg!(windows) {
            "\"workspace with 'quotes'\""
        } else {
            "'workspace with '\\''quotes'\\'''"
        };
        assert_eq!(
            shell_quote_path(Path::new("workspace with 'quotes'")),
            expected
        );
    }
}
