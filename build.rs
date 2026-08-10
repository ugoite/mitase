use std::{
    collections::BTreeSet,
    env,
    path::{Path, PathBuf},
};

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=MITASE_BUILD_VERSION");
    emit_build_version();
}

fn emit_build_version() {
    let version =
        env::var("MITASE_BUILD_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=MITASE_GIT_VERSION={version}");
}

// Keep the historical build-script symbols stable for the specification inventory. These
// helpers are intentionally inert: Cargo no longer watches Git metadata or invokes git.
#[allow(dead_code)]
fn emit_git_watchers(_manifest_dir: &Path) {}

#[allow(dead_code)]
fn emit_watch(_path: &Path) {}

#[allow(dead_code)]
fn emit_watch_recursive(_path: &Path) {}

#[allow(dead_code)]
fn watch_git_metadata(_path: &Path, _watched: &mut BTreeSet<PathBuf>) {}

#[allow(dead_code)]
fn git_tag_version() -> Option<String> {
    None
}

#[allow(dead_code)]
fn git_dir(_manifest_dir: &Path) -> Option<PathBuf> {
    None
}

#[allow(dead_code)]
fn git_common_dir(_manifest_dir: &Path) -> Option<PathBuf> {
    None
}

#[allow(dead_code)]
fn git_metadata_dir(_manifest_dir: &Path, _suffix: &str) -> Option<PathBuf> {
    None
}
