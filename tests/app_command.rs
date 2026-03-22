use assert_cmd::cargo::CommandCargoExt;
use std::{
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    process::{Child, Command, Stdio},
    thread,
    time::Duration,
};

fn fixture_path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/workspaces")
        .join(name)
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
        if let Ok(response) = http_get(port, "/healthz")
            && response.contains("\r\n\r\nok")
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
