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
