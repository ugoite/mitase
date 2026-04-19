use std::{path::PathBuf, process::Command};

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn run_python_assertions(script: &str) {
    let output = Command::new("python3")
        .arg("-c")
        .arg(script)
        .env("SYU_REPO_ROOT", repo_root())
        .output()
        .expect("python should run");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
// REQ-CORE-006
fn coverage_summary_script_distinguishes_empty_and_uninstrumented_states() {
    run_python_assertions(
        r#"
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import os

repo_root = Path(os.environ["SYU_REPO_ROOT"])
script_path = repo_root / "scripts/ci/write-spec-coverage-summary.py"
spec = spec_from_file_location("spec_coverage_summary", script_path)
module = module_from_spec(spec)
spec.loader.exec_module(module)

assert module.coverage_label(
    total_refs=0,
    rust_file_count=0,
    instrumented_paths=0,
    covered=0,
    total=0,
    empty_label="no implementation refs",
) == "no implementation refs"
assert module.coverage_label(
    total_refs=1,
    rust_file_count=0,
    instrumented_paths=0,
    covered=0,
    total=0,
    empty_label="no implementation refs",
) == "no Rust files"
assert module.coverage_label(
    total_refs=1,
    rust_file_count=1,
    instrumented_paths=0,
    covered=0,
    total=0,
    empty_label="no implementation refs",
) == "not instrumented"
assert module.coverage_label(
    total_refs=1,
    rust_file_count=1,
    instrumented_paths=1,
    covered=0,
    total=0,
    empty_label="no implementation refs",
) == "0.0% (0/0)"
assert module.coverage_label(
    total_refs=1,
    rust_file_count=1,
    instrumented_paths=1,
    covered=3,
    total=4,
    empty_label="no implementation refs",
) == "75.0% (3/4)"
"#,
    );
}

#[test]
// REQ-CORE-006
fn coverage_summary_script_counts_instrumented_paths_separately() {
    run_python_assertions(
        r#"
from importlib.util import module_from_spec, spec_from_file_location
from pathlib import Path
import os

repo_root = Path(os.environ["SYU_REPO_ROOT"])
script_path = repo_root / "scripts/ci/write-spec-coverage-summary.py"
spec = spec_from_file_location("spec_coverage_summary", script_path)
module = module_from_spec(spec)
spec.loader.exec_module(module)

covered, total, instrumented_paths = module.summarize_paths(
    repo_root,
    {
        str(repo_root / "src/a.rs"): (0, 0),
        str(repo_root / "src/b.rs"): (2, 4),
    },
    ["src/a.rs", "src/a.rs", "src/b.rs", "src/c.rs"],
)
assert (covered, total, instrumented_paths) == (2, 4, 2)
"#,
    );
}
