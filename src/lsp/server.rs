// FEAT-LSP-001
// REQ-CORE-001

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;
use std::io::{BufRead, BufReader, Write};

use super::handlers::LspHandlers;
use super::protocol::{LspError, Message, Notification, Request, Response, ResponseError};

pub(crate) struct LspServer {
    handlers: LspHandlers,
    should_exit: bool,
}

impl LspServer {
    pub(crate) fn new() -> Self {
        Self {
            handlers: LspHandlers::new(),
            should_exit: false,
        }
    }

    pub(crate) fn run(&mut self) -> Result<()> {
        let stdin = std::io::stdin();
        let mut stdin = BufReader::new(stdin.lock());
        let mut stdout = std::io::stdout();

        loop {
            if self.should_exit {
                break;
            }

            match self.read_message(&mut stdin)? {
                Some(msg) => {
                    if let Some(response) = self.handle_message(msg)? {
                        self.write_message(&mut stdout, &response)?;
                    }
                }
                None => break,
            }
        }

        Ok(())
    }

    fn read_message<R: BufRead>(&self, reader: &mut R) -> Result<Option<Message>> {
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
            .context("missing Content-Length header")?;

        let mut content = vec![0u8; content_length];
        reader.read_exact(&mut content)?;

        let message: Message = serde_json::from_slice(&content)?;
        Ok(Some(message))
    }

    fn write_message<W: Write>(&self, writer: &mut W, message: &Message) -> Result<()> {
        let json = serde_json::to_string(message)?;
        write!(writer, "Content-Length: {}\r\n\r\n{}", json.len(), json)?;
        writer.flush()?;
        Ok(())
    }

    fn handle_message(&mut self, message: Message) -> Result<Option<Message>> {
        match message {
            Message::Request(req) => Ok(Some(Message::Response(self.handle_request(req)?))),
            Message::Notification(notif) => {
                self.handle_notification(notif)?;
                Ok(None)
            }
            Message::Response(_) => Ok(None),
        }
    }

    fn handle_request(&mut self, request: Request) -> Result<Response> {
        let result: std::result::Result<serde_json::Value, LspError> = match request.method.as_str()
        {
            "initialize" => {
                let params = parse_optional_params(request.params);
                let params = match params {
                    Ok(value) => value,
                    Err(error) => return Ok(error_response(request.id, error)),
                };
                self.handlers.handle_initialize(params)
            }
            "shutdown" => self.handlers.handle_shutdown(),
            "textDocument/hover" => {
                let params = parse_required_params(request.params);
                let params = match params {
                    Ok(value) => value,
                    Err(error) => return Ok(error_response(request.id, error)),
                };
                let hover = self.handlers.handle_hover(params)?;
                serde_json::to_value(hover).map_err(|error| LspError::internal(error.to_string()))
            }
            _ => Err(LspError::method_not_found(request.method)),
        };

        match result {
            Ok(value) => Ok(Response {
                jsonrpc: "2.0".to_string(),
                id: request.id,
                result: Some(value),
                error: None,
            }),
            Err(e) => Ok(error_response(request.id, e)),
        }
    }

    fn handle_notification(&mut self, notification: Notification) -> Result<()> {
        match notification.method.as_str() {
            "initialized" => self.handlers.handle_initialized()?,
            "exit" => {
                self.should_exit = true;
            }
            _ => {}
        }
        Ok(())
    }
}

fn error_response(id: super::protocol::RequestId, error: LspError) -> Response {
    Response {
        jsonrpc: "2.0".to_string(),
        id,
        result: None,
        error: Some(ResponseError::from(error)),
    }
}

fn parse_optional_params<T>(params: Option<serde_json::Value>) -> std::result::Result<T, LspError>
where
    T: DeserializeOwned + Default,
{
    params
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| LspError::invalid_params(error.to_string()))
        .map(Option::unwrap_or_default)
}

fn parse_required_params<T>(params: Option<serde_json::Value>) -> std::result::Result<T, LspError>
where
    T: DeserializeOwned,
{
    let params = params.ok_or_else(|| LspError::invalid_params("missing params"))?;
    serde_json::from_value(params).map_err(|error| LspError::invalid_params(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::{fs, io::Cursor, path::PathBuf};
    use tempfile::tempdir;

    fn fixture_path(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/workspaces")
            .join(name)
    }

    fn message_bytes(value: serde_json::Value) -> Vec<u8> {
        let json = serde_json::to_string(&value).expect("serialize message");
        format!("Content-Length: {}\r\n\r\n{}", json.len(), json).into_bytes()
    }

    #[test]
    fn read_message_returns_none_on_eof() {
        let server = LspServer::new();
        let mut reader = Cursor::new(Vec::<u8>::new());
        assert!(
            server
                .read_message(&mut reader)
                .expect("read eof")
                .is_none()
        );
    }

    #[test]
    fn read_message_requires_content_length_header() {
        let server = LspServer::new();
        let mut reader = Cursor::new(b"Header: nope\r\n\r\n{}".to_vec());
        let error = server
            .read_message(&mut reader)
            .expect_err("missing header should fail");
        assert!(error.to_string().contains("missing Content-Length header"));
    }

    #[test]
    fn write_message_serializes_lsp_envelope() {
        let server = LspServer::new();
        let mut output = Vec::new();
        server
            .write_message(
                &mut output,
                &Message::Response(Response {
                    jsonrpc: "2.0".to_string(),
                    id: super::super::protocol::RequestId::Number(1),
                    result: Some(json!({"ok": true})),
                    error: None,
                }),
            )
            .expect("write should succeed");
        let text = String::from_utf8(output).expect("utf8 output");
        assert!(text.starts_with("Content-Length: "));
        assert!(text.contains("\"ok\":true"));
    }

    #[test]
    fn handle_message_ignores_responses() {
        let mut server = LspServer::new();
        let response = Message::Response(Response {
            jsonrpc: "2.0".to_string(),
            id: super::super::protocol::RequestId::Number(1),
            result: Some(json!(null)),
            error: None,
        });
        assert!(
            server
                .handle_message(response)
                .expect("response handling")
                .is_none()
        );
    }

    #[test]
    fn handle_request_returns_error_for_unknown_methods() {
        let mut server = LspServer::new();
        let response = server
            .handle_request(Request {
                jsonrpc: "2.0".to_string(),
                id: super::super::protocol::RequestId::Number(1),
                method: "workspace/unknown".to_string(),
                params: None,
            })
            .expect("request should produce response");
        assert!(response.result.is_none());
        let error = response.error.expect("error response");
        assert_eq!(
            error.code,
            super::super::protocol::error_codes::METHOD_NOT_FOUND
        );
        assert_eq!(error.message, "method not found: workspace/unknown");
    }

    #[test]
    fn handle_request_returns_error_when_hover_params_are_missing() {
        let mut server = LspServer::new();
        let response = server
            .handle_request(Request {
                jsonrpc: "2.0".to_string(),
                id: super::super::protocol::RequestId::Number(2),
                method: "textDocument/hover".to_string(),
                params: None,
            })
            .expect("missing params should produce an error response");
        let error = response.error.expect("error response");
        assert_eq!(
            error.code,
            super::super::protocol::error_codes::INVALID_PARAMS
        );
        assert_eq!(error.message, "missing params");
    }

    #[test]
    fn handle_request_returns_invalid_params_for_bad_initialize_uri() {
        let mut server = LspServer::new();
        let response = server
            .handle_request(Request {
                jsonrpc: "2.0".to_string(),
                id: super::super::protocol::RequestId::Number(3),
                method: "initialize".to_string(),
                params: Some(json!({
                    "rootUri": "file://%"
                })),
            })
            .expect("bad params should produce an error response");
        let error = response.error.expect("error response");
        assert_eq!(
            error.code,
            super::super::protocol::error_codes::INVALID_PARAMS
        );
        assert!(error.message.contains("invalid file URI"));
    }

    #[test]
    fn handle_request_returns_invalid_params_for_malformed_initialize_payload() {
        let mut server = LspServer::new();
        let response = server
            .handle_request(Request {
                jsonrpc: "2.0".to_string(),
                id: super::super::protocol::RequestId::Number(4),
                method: "initialize".to_string(),
                params: Some(json!({
                    "rootUri": 42
                })),
            })
            .expect("bad params should produce an error response");
        let error = response.error.expect("error response");
        assert_eq!(
            error.code,
            super::super::protocol::error_codes::INVALID_PARAMS
        );
        assert!(error.message.contains("invalid type"));
    }

    #[test]
    fn handle_request_serializes_hover_results() {
        let tempdir = tempdir().expect("tempdir");
        let hover_file = tempdir.path().join("hover.txt");
        fs::write(&hover_file, "REQ-TRACE-001\n").expect("hover file");

        let mut server = LspServer::new();
        server
            .handle_request(Request {
                jsonrpc: "2.0".to_string(),
                id: super::super::protocol::RequestId::Number(1),
                method: "initialize".to_string(),
                params: Some(json!({
                    "rootUri": format!("file://{}", fixture_path("passing").display())
                })),
            })
            .expect("initialize response");

        let response = server
            .handle_request(Request {
                jsonrpc: "2.0".to_string(),
                id: super::super::protocol::RequestId::Number(2),
                method: "textDocument/hover".to_string(),
                params: Some(json!({
                    "textDocument": {"uri": format!("file://{}", hover_file.display())},
                    "position": {"line": 0, "character": 2}
                })),
            })
            .expect("hover response");

        assert!(response.error.is_none());
        assert_eq!(
            response.result.expect("hover result")["contents"]["kind"],
            "markdown"
        );
    }

    #[test]
    fn handle_notification_updates_server_exit_state() {
        let mut server = LspServer::new();
        server
            .handle_notification(Notification {
                jsonrpc: "2.0".to_string(),
                method: "exit".to_string(),
                params: None,
            })
            .expect("exit notification");
        assert!(server.should_exit);
    }

    #[test]
    fn handle_notification_accepts_initialized() {
        let mut server = LspServer::new();
        server
            .handle_notification(Notification {
                jsonrpc: "2.0".to_string(),
                method: "initialized".to_string(),
                params: Some(json!({})),
            })
            .expect("initialized notification");
        assert!(!server.should_exit);
    }

    #[test]
    fn handle_notification_ignores_unknown_methods() {
        let mut server = LspServer::new();
        server
            .handle_notification(Notification {
                jsonrpc: "2.0".to_string(),
                method: "workspace/didChangeConfiguration".to_string(),
                params: None,
            })
            .expect("unknown notification should be ignored");
        assert!(!server.should_exit);
    }

    #[test]
    fn read_message_round_trips_requests() {
        let server = LspServer::new();
        let bytes = message_bytes(json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "shutdown"
        }));
        let mut reader = Cursor::new(bytes);
        let message = server
            .read_message(&mut reader)
            .expect("message should parse");
        assert!(matches!(
            message,
            Some(Message::Request(Request { method, .. })) if method == "shutdown"
        ));
    }
}
