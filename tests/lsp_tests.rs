// FEAT-LSP-001
// REQ-CORE-001

use serde_json::json;
use std::io::{BufRead, BufReader, Write};
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

#[test]
fn test_lsp_lifecycle() {
    let mut child = Command::new(env!("CARGO_BIN_EXE_syu"))
        .arg("lsp")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("failed to spawn syu lsp");

    let mut stdin = child.stdin.take().expect("failed to get stdin");
    let stdout = child.stdout.take().expect("failed to get stdout");
    let mut reader = BufReader::new(stdout);

    let (tx, rx) = mpsc::channel();

    thread::spawn(move || {
        let mut responses = Vec::new();
        loop {
            match read_message(&mut reader) {
                Ok(Some(msg)) => {
                    responses.push(msg);
                    if responses.len() >= 2 {
                        let _ = tx.send(responses);
                        break;
                    }
                }
                Ok(None) => break,
                Err(e) => {
                    eprintln!("Error reading message: {}", e);
                    break;
                }
            }
        }
    });

    let initialize_request = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": {
            "processId": null,
            "rootUri": null,
            "capabilities": {}
        }
    });

    send_message(&mut stdin, &initialize_request).expect("failed to send initialize");

    let initialized_notification = json!({
        "jsonrpc": "2.0",
        "method": "initialized",
        "params": {}
    });

    send_message(&mut stdin, &initialized_notification).expect("failed to send initialized");

    let shutdown_request = json!({
        "jsonrpc": "2.0",
        "id": 2,
        "method": "shutdown"
    });

    send_message(&mut stdin, &shutdown_request).expect("failed to send shutdown");

    let exit_notification = json!({
        "jsonrpc": "2.0",
        "method": "exit"
    });

    send_message(&mut stdin, &exit_notification).expect("failed to send exit");

    drop(stdin);

    let responses = rx
        .recv_timeout(Duration::from_secs(5))
        .expect("did not receive responses in time");

    assert!(responses.len() >= 2, "expected at least 2 responses");

    let init_response = &responses[0];
    assert_eq!(init_response["jsonrpc"], "2.0");
    assert_eq!(init_response["id"], 1);
    assert!(
        init_response["result"]["capabilities"]["hoverProvider"]
            .as_bool()
            .unwrap_or(false)
    );

    let shutdown_response = &responses[1];
    assert_eq!(shutdown_response["jsonrpc"], "2.0");
    assert_eq!(shutdown_response["id"], 2);

    let _ = child.wait();
}

#[test]
fn test_lsp_exits_cleanly_on_immediate_eof() {
    let output = Command::new(env!("CARGO_BIN_EXE_syu"))
        .arg("lsp")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("failed to run syu lsp");

    assert!(
        output.status.success(),
        "stdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn send_message<W: Write>(writer: &mut W, message: &serde_json::Value) -> std::io::Result<()> {
    let json = serde_json::to_string(message)?;
    write!(writer, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
    writer.flush()?;
    Ok(())
}

fn read_message<R: BufRead>(reader: &mut R) -> std::io::Result<Option<serde_json::Value>> {
    let mut headers = Vec::new();
    let mut line = String::new();

    loop {
        line.clear();
        let bytes_read = reader.read_line(&mut line)?;
        if bytes_read == 0 {
            return Ok(None);
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        headers.push(trimmed.to_string());
    }

    let content_length = headers
        .iter()
        .find_map(|h| {
            h.strip_prefix("Content-Length: ")
                .and_then(|s| s.parse::<usize>().ok())
        })
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "missing Content-Length")
        })?;

    let mut content = vec![0u8; content_length];
    reader.read_exact(&mut content)?;

    let message: serde_json::Value = serde_json::from_slice(&content)?;
    Ok(Some(message))
}
