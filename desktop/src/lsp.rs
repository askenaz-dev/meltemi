// SPDX-License-Identifier: Apache-2.0

//! BYO-LSP (gui-tauri-paridad design D5): the desktop client speaks the
//! Language Server Protocol with servers the USER already has installed —
//! nothing is bundled (installer budget, constitution §10). Detection walks
//! PATH for well-known binaries; without a server the editor degrades to
//! syntax highlight, honestly announced. LSP is surface intelligence, not a
//! daemon capability: writes still go through `worktree/apply-edit`.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicI64, Ordering};

use serde_json::{Value, json};
use tauri::Emitter;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{Mutex, oneshot};

use crate::bridge::BridgeError;
use crate::fsops::find_on_path;

/// Well-known BYO servers per LSP language id. The user can point at any other
/// server — or override one of these — through the persisted UI state
/// (`lspServers`), which is what "instalado **o configurado**" asks for: nothing
/// is bundled either way, the binary is always the user's.
const SERVERS: &[(&str, &[&str])] = &[
    ("rust", &["rust-analyzer"]),
    ("typescript", &["typescript-language-server", "--stdio"]),
    ("python", &["pyright-langserver", "--stdio"]),
];

/// The command for a language: the user's configured one first, then the
/// well-known default. `None` when neither exists.
fn command_for(app: &tauri::AppHandle, language: &str) -> Option<Vec<String>> {
    if let Some(configured) = crate::uistate::configured_lsp(app, language)
        && !configured.is_empty()
    {
        return Some(configured);
    }
    SERVERS
        .iter()
        .find(|(lang, _)| *lang == language)
        .map(|(_, argv)| argv.iter().map(|part| (*part).to_string()).collect())
}

fn refused(detail: impl Into<String>) -> BridgeError {
    BridgeError {
        code: None,
        message: "lsp".into(),
        kind: "lsp".into(),
        detail: Some(detail.into()),
        remedy: None,
        candidates: None,
    }
}

/// A `file://` URI for a file under the tree (forward slashes; spaces
/// escaped — enough for local paths).
pub fn file_uri(root: &str, file: &str) -> String {
    let joined = format!("{}/{}", root.trim_end_matches(['/', '\\']), file);
    let normalized = joined.replace('\\', "/").replace(' ', "%20");
    if normalized.starts_with('/') {
        format!("file://{normalized}")
    } else {
        format!("file:///{normalized}")
    }
}

type Pending = Arc<Mutex<HashMap<i64, oneshot::Sender<Value>>>>;

/// One running server (per language id).
#[derive(Clone)]
struct Client {
    server_name: String,
    writer: tokio::sync::mpsc::UnboundedSender<Value>,
    pending: Pending,
    next_id: Arc<AtomicI64>,
    versions: Arc<Mutex<HashMap<String, i64>>>,
}

impl Client {
    async fn request(&self, method: &str, params: Value) -> Result<Value, BridgeError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(id, tx);
        self.writer
            .send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }))
            .map_err(|_| refused("server gone"))?;
        tokio::time::timeout(std::time::Duration::from_secs(10), rx)
            .await
            .map_err(|_| refused(format!("{method}: timed out")))?
            .map_err(|_| refused("server closed mid-request"))
    }

    fn notify(&self, method: &str, params: Value) {
        let _ = self
            .writer
            .send(json!({ "jsonrpc": "2.0", "method": method, "params": params }));
    }
}

/// The per-app hub of running LSP clients.
#[derive(Default)]
pub struct LspHub {
    clients: Mutex<HashMap<String, Client>>,
}

/// Writes one LSP frame (Content-Length header + JSON body).
async fn write_frame<W: tokio::io::AsyncWrite + Unpin>(
    writer: &mut W,
    value: &Value,
) -> std::io::Result<()> {
    let body = serde_json::to_string(value).expect("LSP value serializes");
    writer
        .write_all(format!("Content-Length: {}\r\n\r\n", body.len()).as_bytes())
        .await?;
    writer.write_all(body.as_bytes()).await?;
    writer.flush().await
}

/// Reads one LSP frame.
async fn read_frame<R: tokio::io::AsyncBufRead + Unpin>(reader: &mut R) -> Option<Value> {
    let mut content_length: usize = 0;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).await.ok()? == 0 {
            return None;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            break;
        }
        if let Some(value) = trimmed.strip_prefix("Content-Length:") {
            content_length = value.trim().parse().ok()?;
        }
    }
    let mut body = vec![0u8; content_length];
    reader.read_exact(&mut body).await.ok()?;
    serde_json::from_slice(&body).ok()
}

async fn spawn_client(
    app: tauri::AppHandle,
    language: &str,
    root: &str,
) -> Result<Option<Client>, BridgeError> {
    let Some(argv) = command_for(&app, language) else {
        return Ok(None);
    };
    let binary_name = if cfg!(windows) {
        // PATH probing on Windows: the shims are .exe or .cmd.
        [format!("{}.exe", argv[0]), format!("{}.cmd", argv[0])]
            .into_iter()
            .find_map(|candidate| find_on_path(&candidate))
    } else {
        find_on_path(&argv[0])
    };
    let Some(binary) = binary_name else {
        return Ok(None); // BYO: not installed — honest degradation.
    };

    let mut child = tokio::process::Command::new(&binary)
        .args(&argv[1..])
        .current_dir(root)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
        .map_err(|e| refused(format!("{}: {e}", binary.display())))?;

    let mut stdin = child.stdin.take().expect("piped stdin");
    let stdout = child.stdout.take().expect("piped stdout");

    let (writer_tx, mut writer_rx) = tokio::sync::mpsc::unbounded_channel::<Value>();
    tokio::spawn(async move {
        while let Some(value) = writer_rx.recv().await {
            if write_frame(&mut stdin, &value).await.is_err() {
                break;
            }
        }
    });

    let pending: Pending = Arc::default();
    let reader_pending = Arc::clone(&pending);
    tokio::spawn(async move {
        let mut reader = BufReader::new(stdout);
        while let Some(message) = read_frame(&mut reader).await {
            if let Some(id) = message.get("id").and_then(Value::as_i64)
                && message.get("method").is_none()
            {
                if let Some(tx) = reader_pending.lock().await.remove(&id) {
                    let _ = tx.send(message.get("result").cloned().unwrap_or(Value::Null));
                }
            } else if message.get("method").and_then(Value::as_str)
                == Some("textDocument/publishDiagnostics")
            {
                let _ = app.emit("lsp:diagnostics", message.get("params").cloned());
            }
            // Server-initiated requests (configuration, registration) are
            // beyond the utilitarian scope; ignoring them keeps servers in
            // their default behavior.
        }
        let _ = child.wait().await;
    });

    let client = Client {
        server_name: argv[0].to_string(),
        writer: writer_tx,
        pending,
        next_id: Arc::new(AtomicI64::new(1)),
        versions: Arc::default(),
    };

    // Handshake: initialize → initialized.
    let root_uri = file_uri(root, "");
    let root_uri = root_uri.trim_end_matches('/');
    client
        .request(
            "initialize",
            json!({
                "processId": std::process::id(),
                "rootUri": root_uri,
                "capabilities": {
                    "textDocument": {
                        "publishDiagnostics": {},
                        "completion": { "completionItem": { "snippetSupport": false } },
                        "definition": {}
                    }
                }
            }),
        )
        .await?;
    client.notify("initialized", json!({}));
    Ok(Some(client))
}

/// Ensures a server for `language` runs (spawning it on first use). Returns
/// the server's name, or `None` when the user has none installed.
#[tauri::command]
pub async fn lsp_ensure(
    app: tauri::AppHandle,
    hub: tauri::State<'_, LspHub>,
    root: String,
    language: String,
) -> Result<Option<String>, BridgeError> {
    let mut clients = hub.clients.lock().await;
    if let Some(client) = clients.get(&language) {
        return Ok(Some(client.server_name.clone()));
    }
    match spawn_client(app, &language, &root).await? {
        Some(client) => {
            let name = client.server_name.clone();
            clients.insert(language, client);
            Ok(Some(name))
        }
        None => Ok(None),
    }
}

/// `textDocument/didOpen`.
#[tauri::command]
pub async fn lsp_open(
    hub: tauri::State<'_, LspHub>,
    root: String,
    file: String,
    language: String,
    content: String,
) -> Result<(), BridgeError> {
    let clients = hub.clients.lock().await;
    let Some(client) = clients.get(&language) else {
        return Ok(());
    };
    let uri = file_uri(&root, &file);
    client.versions.lock().await.insert(uri.clone(), 1);
    client.notify(
        "textDocument/didOpen",
        json!({ "textDocument": {
            "uri": uri, "languageId": language, "version": 1, "text": content,
        }}),
    );
    Ok(())
}

/// `textDocument/didChange` (full sync).
#[tauri::command]
pub async fn lsp_change(
    hub: tauri::State<'_, LspHub>,
    root: String,
    file: String,
    language: String,
    content: String,
) -> Result<(), BridgeError> {
    let clients = hub.clients.lock().await;
    let Some(client) = clients.get(&language) else {
        return Ok(());
    };
    let uri = file_uri(&root, &file);
    let version = {
        let mut versions = client.versions.lock().await;
        let version = versions.entry(uri.clone()).or_insert(1);
        *version += 1;
        *version
    };
    client.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": uri, "version": version },
            "contentChanges": [{ "text": content }],
        }),
    );
    Ok(())
}

/// Generic request bridge (completion, definition, …): the webview sends the
/// position params; the Rust side injects the document URI.
#[tauri::command]
pub async fn lsp_request(
    hub: tauri::State<'_, LspHub>,
    root: String,
    file: String,
    language: String,
    method: String,
    params: Value,
) -> Result<Value, BridgeError> {
    let client = {
        let clients = hub.clients.lock().await;
        clients.get(&language).cloned()
    };
    let Some(client) = client else {
        return Ok(Value::Null);
    };
    let mut params = params;
    params["textDocument"] = json!({ "uri": file_uri(&root, &file) });
    client.request(&method, params).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn file_uris_are_forward_slashed_and_rooted() {
        assert_eq!(
            file_uri("C:\\repo", "src/lib.rs"),
            "file:///C:/repo/src/lib.rs"
        );
        assert_eq!(
            file_uri("/home/u/repo", "a b.rs"),
            "file:///home/u/repo/a%20b.rs"
        );
    }

    #[tokio::test]
    async fn frames_roundtrip() {
        let value = json!({ "jsonrpc": "2.0", "id": 1, "method": "x", "params": {} });
        let mut buffer = Vec::new();
        write_frame(&mut buffer, &value).await.expect("write");
        let mut reader = BufReader::new(buffer.as_slice());
        let back = read_frame(&mut reader).await.expect("read");
        assert_eq!(back, value);
    }
}
