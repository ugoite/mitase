use std::{fs, path::Path, process::Command};

pub fn isolated_fixture(name: &str) -> tempfile::TempDir {
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures/v1")
        .join(name);
    let temp = tempfile::tempdir().expect("fixture tempdir");
    copy_fixture_tree(&fixture, temp.path());
    initialize_fixture_git(temp.path());
    temp
}

fn copy_fixture_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("fixture destination");
    for entry in fs::read_dir(source).expect("fixture directory") {
        let entry = entry.expect("fixture entry");
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_fixture_tree(&source_path, &destination_path);
        } else {
            fs::copy(source_path, destination_path).expect("copy fixture file");
        }
    }
}

fn initialize_fixture_git(root: &Path) {
    for args in [
        ["init", "-q"].as_slice(),
        ["config", "user.email", "syu-tests@example.invalid"].as_slice(),
        ["config", "user.name", "Syu Tests"].as_slice(),
        ["add", "."].as_slice(),
        ["commit", "-qm", "fixture baseline"].as_slice(),
    ] {
        assert!(
            Command::new("git")
                .args(args)
                .current_dir(root)
                .status()
                .expect("run fixture git command")
                .success()
        );
    }
}
