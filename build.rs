use std::env;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-env-changed=SYU_BUILD_VERSION");
    emit_build_version();
}

fn emit_build_version() {
    let version =
        env::var("SYU_BUILD_VERSION").unwrap_or_else(|_| env!("CARGO_PKG_VERSION").to_string());
    println!("cargo:rustc-env=SYU_GIT_VERSION={version}");
}
