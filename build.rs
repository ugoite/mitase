use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").expect("manifest dir"));

    println!("cargo:rerun-if-changed=build.rs");
    emit_git_watchers(&manifest_dir);
    emit_watch(&manifest_dir.join("Cargo.lock"));
    emit_build_version();
}

fn emit_watch(path: &Path) {
    if path.exists() {
        println!("cargo:rerun-if-changed={}", path.display());
    }
}

fn emit_watch_recursive(path: &Path) {
    if !path.exists() {
        return;
    }

    if path.is_file() {
        emit_watch(path);
        return;
    }

    for entry in fs::read_dir(path).expect("watch directory should be readable") {
        let child = entry.expect("watch directory entry should exist").path();
        if child.is_dir() {
            emit_watch_recursive(&child);
        } else {
            emit_watch(&child);
        }
    }
}

fn emit_git_watchers(manifest_dir: &Path) {
    let git_dir = git_dir(manifest_dir);
    let git_common_dir = git_common_dir(manifest_dir);
    let mut watched = BTreeSet::new();

    if let Some(path) = git_dir.as_deref() {
        watch_git_metadata(path, &mut watched);
    }

    if let Some(path) = git_common_dir.as_deref()
        && Some(path) != git_dir.as_deref()
    {
        watch_git_metadata(path, &mut watched);
    }
}

fn watch_git_metadata(path: &Path, watched: &mut BTreeSet<PathBuf>) {
    for candidate in [path.join("HEAD"), path.join("packed-refs")] {
        if watched.insert(candidate.clone()) {
            emit_watch(&candidate);
        }
    }

    let refs_dir = path.join("refs");
    if watched.insert(refs_dir.clone()) {
        emit_watch_recursive(&refs_dir);
    }
}

fn emit_build_version() {
    let version = git_tag_version().unwrap_or_else(|| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=SYU_GIT_VERSION={version}");
}

fn git_tag_version() -> Option<String> {
    Command::new("git")
        .args(["describe", "--tags", "--exact-match"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

fn git_dir(manifest_dir: &Path) -> Option<PathBuf> {
    git_metadata_dir(manifest_dir, ".git")
}

fn git_common_dir(manifest_dir: &Path) -> Option<PathBuf> {
    git_metadata_dir(manifest_dir, ".git/modules")
}

fn git_metadata_dir(manifest_dir: &Path, suffix: &str) -> Option<PathBuf> {
    let candidate = manifest_dir.join(suffix);
    candidate.is_dir().then_some(candidate)
}
