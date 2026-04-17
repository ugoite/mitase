use assert_cmd::cargo::CommandCargoExt;
use std::{
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::{Path, PathBuf},
    process::{Child, Command, Output, Stdio},
    thread,
    time::Duration,
};
use tempfile::tempdir;

fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
}

fn copy_dir_recursive(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).expect("destination dir");
    for entry in fs::read_dir(source).expect("source dir") {
        let entry = entry.expect("dir entry");
        let entry_type = entry.file_type().expect("entry type");
        let destination_path = destination.join(entry.file_name());
        if entry_type.is_dir() {
            copy_dir_recursive(&entry.path(), &destination_path);
        } else {
            fs::copy(entry.path(), destination_path).expect("file copy");
        }
    }
}

fn configured_workspace(bind: &str, port: u16) -> (tempfile::TempDir, PathBuf) {
    let tempdir = tempdir().expect("tempdir should exist");
    let workspace = tempdir.path().join("workspace");
    copy_dir_recursive(&fixture_path("passing"), &workspace);
    fs::write(
        workspace.join("syu.yaml"),
        format!(
            "version: {version}\nspec:\n  root: docs/syu\nvalidate:\n  default_fix: false\n  allow_planned: true\n  require_non_orphaned_items: true\n  require_symbol_trace_coverage: false\napp:\n  bind: {bind}\n  port: {port}\nruntimes:\n  python:\n    command: auto\n  node:\n    command: auto\n",
            version = env!("CARGO_PKG_VERSION"),
        ),
    )
    .expect("config");
    (tempdir, workspace)
}

fn reserve_port() -> u16 {
    let listener = TcpListener::bind(("127.0.0.1", 0)).expect("listener should bind");
    let port = listener
        .local_addr()
        .expect("local addr should resolve")
        .port();
    drop(listener);
    port
}

fn wait_for_server(port: u16) {
    for _ in 0..80 {
        if let Ok(response) = http_get(port, "/health")
            && response.contains("200 OK")
            && response.contains("\"status\":\"ok\"")
        {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }

    panic!("server did not become ready");
}

fn http_get(port: u16, path: &str) -> std::io::Result<String> {
    let mut stream = TcpStream::connect(("127.0.0.1", port))?;
    write!(
        stream,
        "GET {path} HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n"
    )?;
    stream.flush()?;

    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn shutdown_child(child: &mut Child) {
    #[cfg(unix)]
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }

    #[cfg(not(unix))]
    child.kill().expect("child should terminate");

    let status = child.wait().expect("child should exit");
    assert!(status.success(), "app command should exit cleanly");
}

fn shutdown_child_with_output(child: Child) -> Output {
    #[cfg(unix)]
    unsafe {
        libc::kill(child.id() as i32, libc::SIGINT);
    }

    #[cfg(not(unix))]
    child.kill().expect("child should terminate");

    let output = child.wait_with_output().expect("child should exit");
    assert!(output.status.success(), "app command should exit cleanly");
    output
}

fn wait_for_output_fragment(path: &Path, fragment: &str) {
    for _ in 0..80 {
        if fs::read_to_string(path)
            .map(|contents| contents.contains(fragment))
            .unwrap_or(false)
        {
            return;
        }
        thread::sleep(Duration::from_millis(100));
    }

    panic!("process output did not contain {fragment:?}");
}

// REQ-CORE-017
#[test]
fn app_command_serves_browser_ui_and_payload() {
    let port = reserve_port();
    let mut child = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(fixture_path("passing"))
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("app command should start");

    wait_for_server(port);

    let index = http_get(port, "/").expect("index should load");
    assert!(index.contains("200 OK"));
    assert!(index.contains("<!doctype html>"));
    assert!(index.contains("syu app"));

    let payload = http_get(port, "/api/app-data.json").expect("payload should load");
    assert!(payload.contains("200 OK"));
    assert!(payload.contains("REQ-TRACE-001"));
    assert!(payload.contains("FEAT-TRACE-001"));
    assert!(payload.contains("foundation.yaml"));
    assert!(
        !payload.contains(&fixture_path("passing").display().to_string()),
        "browser payload should not expose the absolute workspace path",
    );
    assert!(
        !payload.contains(
            &fixture_path("passing")
                .join("docs/syu")
                .display()
                .to_string()
        ),
        "browser payload should not expose the absolute spec path",
    );

    let health = http_get(port, "/health").expect("health should load");
    assert!(health.contains("200 OK"));
    assert!(health.contains("\"status\":\"ok\""));
    assert!(health.contains(&format!("\"version\":\"{}\"", env!("CARGO_PKG_VERSION"))));

    shutdown_child(&mut child);
}

#[test]
fn app_command_startup_message_explains_browser_and_stop_flow() {
    let port = reserve_port();
    let tempdir = tempdir().expect("tempdir should exist");
    let stdout_path = tempdir.path().join("stdout.log");
    let stderr_path = tempdir.path().join("stderr.log");
    let stdout = fs::File::create(&stdout_path).expect("stdout log should open");
    let stderr = fs::File::create(&stderr_path).expect("stderr log should open");
    let mut child = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(fixture_path("passing"))
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::from(stdout))
        .stderr(Stdio::from(stderr))
        .spawn()
        .expect("app command should start");

    wait_for_server(port);
    wait_for_output_fragment(&stdout_path, "Press Ctrl-C to stop.");

    shutdown_child(&mut child);

    let stdout = fs::read_to_string(&stdout_path).expect("stdout should be readable");
    let stderr = fs::read_to_string(&stderr_path).expect("stderr should be readable");
    assert!(stdout.contains(&format!("syu app listening on http://127.0.0.1:{port}")));
    assert!(stdout.contains(&format!("syu app ready: http://127.0.0.1:{port}")));
    assert!(stdout.contains(&format!("Open http://127.0.0.1:{port} in your browser.")));
    assert!(stdout.contains("Press Ctrl-C to stop."));
    assert!(
        !stderr.contains("warning: syu app is bound"),
        "loopback binds should not print a public exposure warning",
    );
}

#[test]
fn app_command_help_mentions_browser_and_stop_instructions() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg("--help")
        .output()
        .expect("help should render");

    assert!(output.status.success(), "help should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Start a local HTTP server and browser UI for workspace exploration"));
    assert!(stdout.contains("print the URL to open in your browser"));
    assert!(stdout.contains("Use GET /health for readiness checks once the app is serving."));
    assert!(stdout.contains("After startup, open the printed URL in your browser."));
    assert!(stdout.contains("Press Ctrl-C to stop the local app server."));
}

#[test]
fn app_command_warns_on_non_loopback_binds() {
    let port = reserve_port();
    let child = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(fixture_path("passing"))
        .arg("--bind")
        .arg("0.0.0.0")
        .arg("--port")
        .arg(port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("app command should start");

    wait_for_server(port);

    let output = shutdown_child_with_output(child);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let warning = "warning: syu app is bound to 0.0.0.0";
    let listening = format!("syu app listening on http://0.0.0.0:{port}");
    let warning_index = stdout
        .find(warning)
        .expect("stdout should include the public bind warning");
    let listening_index = stdout
        .find(&listening)
        .expect("stdout should include the listening url");
    assert!(stdout.contains(&format!("syu app listening on http://0.0.0.0:{port}")));
    assert!(stdout.contains(&format!("syu app ready: http://0.0.0.0:{port}")));
    assert!(stdout.contains(&format!("Open http://0.0.0.0:{port} in your browser.")));
    assert!(warning_index < listening_index);
    assert!(stdout.contains("workspace data and source documents may be reachable"));
    assert!(stdout.contains("use --bind 127.0.0.1 to keep the browser UI local"));
    assert!(
        stderr.trim().is_empty(),
        "startup warnings should stay on stdout"
    );
}

#[test]
fn app_command_uses_configured_bind_and_port_defaults() {
    let port = reserve_port();
    let (_tempdir, workspace) = configured_workspace("127.0.0.1", port);

    let mut child = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(&workspace)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("app command should start");

    wait_for_server(port);
    let health = http_get(port, "/health").expect("health should load");
    assert!(health.contains("\"status\":\"ok\""));
    let health = http_get(port, "/healthz").expect("healthz should load");
    assert!(health.contains("200 OK"));

    shutdown_child(&mut child);
}

#[test]
fn app_command_rejects_invalid_bind_addresses() {
    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(fixture_path("passing"))
        .arg("--bind")
        .arg("definitely-not-an-ip")
        .output()
        .expect("command should run");

    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid bind address"));
}

#[test]
fn app_command_rejects_invalid_bind_addresses_from_config() {
    let (_tempdir, workspace) = configured_workspace("definitely-not-an-ip", 3000);

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(&workspace)
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "invalid config bind should fail");
    assert!(String::from_utf8_lossy(&output.stderr).contains("invalid bind address"));
}

#[test]
fn app_command_explains_how_to_recover_from_port_binding_failures() {
    let occupied = TcpListener::bind(("127.0.0.1", 0)).expect("occupied listener should bind");
    let port = occupied
        .local_addr()
        .expect("occupied listener address should resolve")
        .port();

    let output = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(fixture_path("passing"))
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(port.to_string())
        .output()
        .expect("command should run");

    assert!(!output.status.success(), "bind conflict should fail");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains(&format!("failed to bind `127.0.0.1:{port}`")));
    assert!(stderr.contains("selected port is likely already in use"));
    assert!(stderr.contains(&format!(
        "syu app {} --port <free-port>",
        fixture_path("passing").display()
    )));
    assert!(stderr.contains("app.port"));
}

#[test]
fn app_command_cli_flags_override_configured_bind_and_port() {
    let override_port = reserve_port();
    let (_tempdir, workspace) = configured_workspace("definitely-not-an-ip", 39999);

    let mut child = Command::cargo_bin("syu")
        .expect("binary should build")
        .arg("app")
        .arg(&workspace)
        .arg("--bind")
        .arg("127.0.0.1")
        .arg("--port")
        .arg(override_port.to_string())
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("app command should start");

    wait_for_server(override_port);
    let health = http_get(override_port, "/health").expect("health should load");
    assert!(health.contains("\"status\":\"ok\""));
    let health = http_get(override_port, "/healthz").expect("healthz should load");
    assert!(health.contains("200 OK"));

    shutdown_child(&mut child);
}
