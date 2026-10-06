//! The TypeScript language service — reached through `tsgo --lsp`.
//!
//! The API server ([`super::native`]) carries the checker's primitives —
//! types, symbols, diagnostics, emit — but not the *language-service*
//! surface an editor needs: quick info, go-to-definition, rename with
//! prepare, signature help, completion with per-item resolve. As of
//! typescript-go `c6b013f5` those exist only behind the compiler's own LSP
//! (`internal/lsp`), so that is what this module drives: one `tsgo --lsp`
//! child per project, spoken to over Content-Length framing.
//!
//! This is an implementation detail of the TypeScript adapter, decided
//! feature by feature (see `docs/design/lsp-architecture.md`): nothing
//! above [`crate::engine`]'s semantic API knows an LSP is involved, and if
//! a future API-server release grows these entrypoints this module can be
//! retired without moving the seam.
//!
//! Two behaviors are load-bearing, learned from the editor client this
//! replaces:
//!
//! - **Server-initiated requests must be answered.** tsgo sends
//!   `client/registerCapability` during startup and waits for the reply; a
//!   client that ignores it hangs on every later request. The reader thread
//!   answers them the moment they arrive.
//! - **Documents are the conversation.** A file that exists only as text in
//!   this process — the lowered TypeScript of an `.tt` buffer — is served
//!   with `didOpen`/`didChange` and answered exactly like one on disk.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// How long any one request may take before the client gives up on it. A
/// hung request must not hang the editor, but it is still a failed request —
/// never a successful request whose result happened to be empty.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);

const TT_CONTENT_MAPPER: &str = "@openload28/tt-lang";

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Arrangement {
    inferred_mapper: Option<serde_json::Value>,
}

impl Arrangement {
    pub(crate) fn of_configuration(configured: &[serde_json::Value], config: &Path) -> Arrangement {
        let names_tt = |entry: &serde_json::Value| {
            entry["package"].as_str() == Some(TT_CONTENT_MAPPER)
                && entry["extensions"].as_array().is_some_and(|extensions| {
                    !extensions.is_empty()
                        && extensions
                            .iter()
                            .all(|extension| matches!(extension.as_str(), Some(".tt" | ".ttx")))
                })
        };
        if configured.is_empty() || !configured.iter().all(names_tt) {
            return Arrangement::default();
        }
        Arrangement {
            inferred_mapper: config.parent().and_then(installed_mapper).map(
                |(directory, manifest)| {
                    serde_json::json!({
                        "contributorId": "tt",
                        "extensions": [".tt", ".ttx"],
                        "inferredProjectContribution": {
                            "manifest": {
                                "name": TT_CONTENT_MAPPER,
                                "version": manifest["version"].as_str().unwrap_or("0.0.0"),
                                "exec": manifest["typescript"]["contentMapper"]["exec"],
                                "cwd": directory,
                            },
                        },
                    })
                },
            ),
        }
    }
}

impl Arrangement {
    pub(crate) fn of_project(
        backend: Option<&super::native::NativeBackend>,
        tsconfig: Option<&Path>,
        root: &Path,
    ) -> Arrangement {
        let (Some(backend), Some(config)) = (backend, tsconfig) else {
            return Arrangement::default();
        };
        match backend.configured_mappers(config, root) {
            Ok(configured) => Arrangement::of_configuration(&configured, config),
            Err(_) => Arrangement::default(),
        }
    }
}

fn installed_mapper(from: &Path) -> Option<(PathBuf, serde_json::Value)> {
    let directory = from
        .ancestors()
        .map(|dir| dir.join("node_modules").join(TT_CONTENT_MAPPER))
        .find(|dir| dir.join("package.json").is_file())?;
    let directory = std::fs::canonicalize(&directory).ok()?;
    let manifest: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(directory.join("package.json")).ok()?)
            .ok()?;
    let exec = manifest["typescript"]["contentMapper"]["exec"].as_array()?;
    if exec.is_empty() || !exec.iter().all(serde_json::Value::is_string) {
        return None;
    }
    Some((directory, manifest))
}

/// A running `tsgo --lsp`, and the conversation with it.
pub(crate) struct Service {
    child: Child,
    /// Shared with the reader thread, which answers server-initiated
    /// requests itself so the server never blocks on one.
    stdin: Arc<Mutex<ChildStdin>>,
    /// Responses to *our* requests, forwarded by the reader thread.
    responses: Receiver<Response>,
    next_id: i64,
    /// Versions of the documents we serve, by URI.
    opened: HashMap<String, i64>,
    documents: HashMap<String, String>,
    launch: (PathBuf, PathBuf, Arrangement),
    alive: bool,
    serves_sources: bool,
    exchange: Option<super::content_projection::ProjectionExchange>,
    projections: HashMap<PathBuf, (String, serde_json::Value)>,
    projection_revision: u64,
    semantic_legend: SemanticLegend,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct SemanticLegend {
    pub types: Vec<String>,
    pub modifiers: Vec<String>,
}

pub(crate) const SEMANTIC_TOKEN_TYPES: [&str; 23] = [
    "namespace",
    "type",
    "class",
    "enum",
    "interface",
    "struct",
    "typeParameter",
    "parameter",
    "variable",
    "property",
    "enumMember",
    "event",
    "function",
    "method",
    "macro",
    "keyword",
    "modifier",
    "comment",
    "string",
    "number",
    "regexp",
    "operator",
    "decorator",
];

pub(crate) const SEMANTIC_TOKEN_MODIFIERS: [&str; 11] = [
    "declaration",
    "definition",
    "readonly",
    "static",
    "deprecated",
    "abstract",
    "async",
    "modification",
    "documentation",
    "defaultLibrary",
    "local",
];

/// One answer from the server: the result, or the error it gave instead.
struct Response {
    id: i64,
    result: serde_json::Value,
    error: Option<String>,
}

#[derive(Debug)]
enum ResponseFailure {
    Protocol(String),
    Timeout(String),
    Disconnected,
}

impl std::fmt::Debug for Service {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Service")
            .field("alive", &self.alive)
            .finish()
    }
}

impl Service {
    /// Starts the server and completes the LSP handshake. `root` is the
    /// workspace the server opens.
    pub(crate) fn start(
        binary: &Path,
        root: &Path,
        arrangement: &Arrangement,
    ) -> Result<Service, String> {
        let exchange = if arrangement.inferred_mapper.is_some() {
            Some(super::content_projection::ProjectionExchange::new()?)
        } else {
            None
        };
        let mut command = Command::new(binary);
        if let Some(exchange) = &exchange {
            command.env(super::content_projection::ENVIRONMENT, exchange.directory());
        }
        let mut child = command
            .args(["--lsp", "-stdio"])
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("cannot run {}: {e}", binary.display()))?;
        let stdin = Arc::new(Mutex::new(child.stdin.take().expect("stdin piped")));
        let stdout = child.stdout.take().expect("stdout piped");
        let stderr = child.stderr.take().expect("stderr piped");

        // The server's own log output must not fill its pipe and stall it.
        std::thread::spawn(move || {
            let mut sink = BufReader::new(stderr);
            let mut scratch = [0u8; 4096];
            while matches!(sink.read(&mut scratch), Ok(n) if n > 0) {}
        });

        let (tx, rx) = channel();
        let reader_stdin = Arc::clone(&stdin);
        std::thread::spawn(move || read_loop(stdout, reader_stdin, tx));

        let mut service = Service {
            child,
            stdin,
            responses: rx,
            next_id: 1,
            opened: HashMap::new(),
            documents: HashMap::new(),
            launch: (
                binary.to_path_buf(),
                root.to_path_buf(),
                arrangement.clone(),
            ),
            alive: true,
            serves_sources: arrangement.inferred_mapper.is_some(),
            exchange,
            projections: HashMap::new(),
            projection_revision: 0,
            semantic_legend: SemanticLegend::default(),
        };

        let root_uri = file_uri(root);
        let mut initialize = serde_json::json!({
            "processId": std::process::id(),
            "rootUri": root_uri,
            "workspaceFolders": [{ "uri": root_uri, "name": "tt" }],
            "capabilities": {
                "textDocument": {
                    "synchronization": { "dynamicRegistration": true },
                    "hover": { "contentFormat": ["markdown", "plaintext"] },
                    "definition": {},
                    "references": {},
                    "completion": { "completionItem": {
                        "labelDetailsSupport": true,
                        "tagSupport": { "valueSet": [1] },
                    } },
                    "signatureHelp": {},
                    "documentSymbol": { "hierarchicalDocumentSymbolSupport": true },
                    "rename": { "prepareSupport": true },
                    "semanticTokens": {
                        "requests": { "full": true },
                        "tokenTypes": SEMANTIC_TOKEN_TYPES,
                        "tokenModifiers": SEMANTIC_TOKEN_MODIFIERS,
                        "formats": ["relative"],
                        "multilineTokenSupport": false,
                        "overlappingTokenSupport": false,
                    },
                    // LSP 3.18 `DiagnosticsCapabilities`: without them the
                    // server leaves out related places and the unused /
                    // deprecated tags a suggestion is drawn with.
                    "diagnostic": {
                        "relatedInformation": true,
                        "tagSupport": { "valueSet": [1, 2] },
                    },
                },
                "workspace": { "configuration": true, "workspaceFolders": true },
            },
        });
        if service.serves_sources {
            initialize["initializationOptions"] = serde_json::json!({ "runExternalCode": true });
        }
        let initialized = service.request("initialize", initialize)?;
        let legend = &initialized["capabilities"]["semanticTokensProvider"]["legend"];
        let names = |list: &serde_json::Value| -> Vec<String> {
            list.as_array()
                .into_iter()
                .flatten()
                .filter_map(|name| name.as_str().map(String::from))
                .collect()
        };
        service.semantic_legend = SemanticLegend {
            types: names(&legend["tokenTypes"]),
            modifiers: names(&legend["tokenModifiers"]),
        };
        service.notify("initialized", serde_json::json!({}));
        if let Some(contribution) = &arrangement.inferred_mapper {
            service.request(
                "custom/setContentMapperContributions",
                serde_json::json!({ "contributions": [contribution], "openDocuments": [] }),
            )?;
        }
        Ok(service)
    }

    pub(crate) fn serves_authored_sources(&self) -> bool {
        self.serves_sources
    }

    /// Publish the complete graph before any document notification can cause
    /// TypeScript to resolve a dependency. Its mapper cache is keyed by source
    /// content; a new projection for equal source needs a fresh host instance.
    pub(crate) fn sync_projection_graph(
        &mut self,
        graph: HashMap<PathBuf, (String, serde_json::Value)>,
    ) -> Result<bool, String> {
        if self.projections == graph {
            return Ok(false);
        }
        let restart = graph.iter().any(|(path, (source, response))| {
            self.projections
                .get(path)
                .is_some_and(|(previous_source, previous_response)| {
                    previous_source == source && previous_response != response
                })
        });
        let mut replacement = if restart {
            Some(Self::start(&self.launch.0, &self.launch.1, &self.launch.2)?)
        } else {
            None
        };
        let documents = self.documents.clone();
        let client = replacement.as_mut().unwrap_or(self);
        client.projection_revision += 1;
        let records: Vec<_> = graph
            .iter()
            .map(
                |(path, (source, response))| super::content_projection::ProjectionRecord {
                    protocol: super::content_projection::PROTOCOL,
                    revision: client.projection_revision,
                    path: path.clone(),
                    source: source.clone(),
                    response: response.clone(),
                },
            )
            .collect();
        client
            .exchange
            .as_mut()
            .ok_or("authored projection requires a mapper exchange")?
            .replace(&records)?;
        client.projections = graph;
        if restart {
            for (uri, previous_text) in documents {
                let text = uri_path(&uri)
                    .and_then(|path| {
                        client
                            .projections
                            .get(&path)
                            .map(|(source, _)| source.clone())
                    })
                    .unwrap_or(previous_text);
                client.open(&uri, &text);
            }
        }
        if let Some(replacement) = replacement {
            *self = replacement;
        }
        Ok(restart)
    }

    pub(crate) fn open_source_projection(
        &mut self,
        path: &Path,
        source: &str,
        response: serde_json::Value,
    ) -> Result<bool, String> {
        let changed = self
            .projections
            .get(path)
            .is_none_or(|(text, previous)| text != source || previous != &response);
        let restarted = if changed {
            let mut graph = self.projections.clone();
            graph.insert(path.to_path_buf(), (source.into(), response));
            self.sync_projection_graph(graph)?
        } else {
            false
        };
        let uri = file_uri(path);
        if self.documents.get(&uri).is_none_or(|text| text != source) {
            self.open(&uri, source);
        }
        Ok(restarted)
    }

    pub(crate) fn semantic_legend(&self) -> &SemanticLegend {
        &self.semantic_legend
    }

    /// Whether the server is still there to answer.
    pub(crate) fn alive(&self) -> bool {
        self.alive
    }

    /// Serves `text` as `uri`, whether or not anything is there on disk.
    /// Whole-document sync: the lowered text is regenerated as a whole, so
    /// there is no incremental edit to describe.
    pub(crate) fn open(&mut self, uri: &str, text: &str) {
        let text = crate::error::decoded(text);
        self.documents.insert(uri.to_string(), text.to_string());
        let version = self.opened.get(uri).copied().unwrap_or(0) + 1;
        self.opened.insert(uri.to_string(), version);
        if version == 1 {
            self.notify(
                "textDocument/didOpen",
                serde_json::json!({
                    "textDocument": {
                        "uri": uri, "languageId": language_id(uri),
                        "version": version, "text": text,
                    },
                }),
            );
        } else {
            self.notify(
                "textDocument/didChange",
                serde_json::json!({
                    "textDocument": { "uri": uri, "version": version },
                    "contentChanges": [{ "text": text }],
                }),
            );
        }
    }

    /// Releases an overlay so subsequent requests observe the disk again.
    pub(crate) fn close(&mut self, uri: &str) {
        self.documents.remove(uri);
        if self.opened.remove(uri).is_some() {
            self.notify(
                "textDocument/didClose",
                serde_json::json!({
                    "textDocument": { "uri": uri },
                }),
            );
        }
    }

    /// Asks the server something.
    ///
    /// `Ok(Null)` means the server successfully answered that the feature has
    /// no result. Protocol errors and timeouts remain `Err`: callers already
    /// carry that distinction through the engine and editor boundaries, and
    /// collapsing it here would make a broken type service look like valid
    /// negative type information. A disconnected process also marks the
    /// conversation dead so the next question can start a fresh one.
    pub(crate) fn request(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, String> {
        self.answer(method, params)?.map_err(|error| {
            format!("TypeScript language service request `{method}` failed: {error}")
        })
    }

    pub(crate) fn answer(
        &mut self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<Result<serde_json::Value, String>, String> {
        if !self.alive {
            return Err("the TypeScript server is not running".to_string());
        }
        let id = self.next_id;
        self.next_id += 1;
        self.send(serde_json::json!({
            "jsonrpc": "2.0", "id": id, "method": method, "params": params,
        }))?;

        match wait_for_response(&self.responses, id, method, REQUEST_TIMEOUT) {
            Ok(result) => {
                if method.starts_with("textDocument/")
                    && !self.projections.is_empty()
                    && let Some(exchange) = &self.exchange
                {
                    exchange.verify_acknowledged()?;
                }
                Ok(Ok(result))
            }
            Err(ResponseFailure::Disconnected) => {
                self.alive = false;
                Err("the TypeScript server exited".to_string())
            }
            Err(ResponseFailure::Timeout(error)) => {
                // A request that exceeded the deadline leaves no evidence
                // that this conversation can make progress. Retiring it is
                // what lets the next editor action start a fresh service
                // instead of queuing behind the same hung process forever.
                self.alive = false;
                Err(error)
            }
            Err(ResponseFailure::Protocol(error)) => Ok(Err(error)),
        }
    }

    fn notify(&mut self, method: &str, params: serde_json::Value) {
        let _ = self.send(serde_json::json!({
            "jsonrpc": "2.0", "method": method, "params": params,
        }));
    }

    fn send(&mut self, message: serde_json::Value) -> Result<(), String> {
        let text = message.to_string();
        let mut stdin = self.stdin.lock().expect("stdin lock");
        let framed = format!("Content-Length: {}\r\n\r\n{text}", text.len());
        stdin
            .write_all(framed.as_bytes())
            .and_then(|_| stdin.flush())
            .map_err(|e| {
                self.alive = false;
                format!("the TypeScript server is gone: {e}")
            })
    }
}

/// Waits for one response while discarding replies to requests that already
/// timed out. Kept separate from process ownership so the three protocol
/// outcomes — value, server error, and no response — stay directly testable.
fn wait_for_response(
    responses: &Receiver<Response>,
    id: i64,
    method: &str,
    timeout: Duration,
) -> Result<serde_json::Value, ResponseFailure> {
    let deadline = Instant::now() + timeout;
    loop {
        let remaining = deadline.saturating_duration_since(Instant::now());
        match responses.recv_timeout(remaining) {
            Ok(response) if response.id == id => {
                return match response.error {
                    Some(error) => Err(ResponseFailure::Protocol(error)),
                    None => Ok(response.result),
                };
            }
            Ok(_) => continue,
            Err(RecvTimeoutError::Timeout) => {
                return Err(ResponseFailure::Timeout(format!(
                    "TypeScript language service request `{method}` timed out after {timeout:?}"
                )));
            }
            Err(RecvTimeoutError::Disconnected) => {
                return Err(ResponseFailure::Disconnected);
            }
        }
    }
}

/// LSP document kind for a lowered TypeScript-family module. The projected
/// URI owns this decision: `.tt` is served as `.tt.ts`, `.ttx` as `.ttx.tsx`.
fn language_id(uri: &str) -> &'static str {
    if uri.ends_with(".tsx") || uri.ends_with(".ttx") {
        "typescriptreact"
    } else {
        "typescript"
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Parses frames off the server's stdout: answers server-initiated requests
/// in place, forwards responses to [`Service::request`], drops notifications.
fn read_loop(
    stdout: std::process::ChildStdout,
    stdin: Arc<Mutex<ChildStdin>>,
    responses: Sender<Response>,
) {
    let mut reader = BufReader::new(stdout);
    loop {
        // Headers, ending at an empty line; only Content-Length matters.
        let mut length: Option<usize> = None;
        loop {
            let mut line = String::new();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => return,
                Ok(_) => {}
            }
            let line = line.trim_end();
            if line.is_empty() {
                break;
            }
            if let Some(value) = line.strip_prefix("Content-Length:") {
                length = value.trim().parse().ok();
            }
        }
        let Some(length) = length else { return };
        let mut body = vec![0u8; length];
        if reader.read_exact(&mut body).is_err() {
            return;
        }
        let Ok(message) = serde_json::from_slice::<serde_json::Value>(&body) else {
            continue;
        };

        let id = message.get("id");
        let method = message.get("method").and_then(|m| m.as_str());
        match (id, method) {
            // A response to one of our requests.
            (Some(id), None) => {
                let response = Response {
                    id: id.as_i64().unwrap_or(-1),
                    result: message
                        .get("result")
                        .cloned()
                        .unwrap_or(serde_json::Value::Null),
                    error: message.get("error").map(|error| {
                        error
                            .get("message")
                            .and_then(|message| message.as_str())
                            .map(String::from)
                            .unwrap_or_else(|| error.to_string())
                    }),
                };
                if responses.send(response).is_err() {
                    return;
                }
            }
            // A request from the server. Answering is not optional: tsgo
            // registers capabilities during startup and waits for the reply
            // before serving anything else.
            (Some(id), Some(method)) => {
                let result = if method == "workspace/configuration" {
                    serde_json::json!([{}])
                } else {
                    serde_json::Value::Null
                };
                let reply =
                    serde_json::json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string();
                let mut stdin = stdin.lock().expect("stdin lock");
                let framed = format!("Content-Length: {}\r\n\r\n{reply}", reply.len());
                if stdin
                    .write_all(framed.as_bytes())
                    .and_then(|_| stdin.flush())
                    .is_err()
                {
                    return;
                }
            }
            // A notification (published diagnostics, logs) — not consumed:
            // diagnostics are pulled, and logs are the server's own.
            _ => {}
        }
    }
}

/// The `file://` URI of an absolute path, with the few characters that
/// would break a URI escaped.
pub(crate) fn file_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for byte in path.to_string_lossy().bytes() {
        match byte {
            b' ' => out.push_str("%20"),
            b'%' => out.push_str("%25"),
            b'#' => out.push_str("%23"),
            b'?' => out.push_str("%3F"),
            byte if !byte.is_ascii() => out.push_str(&format!("%{byte:02X}")),
            _ => out.push(byte as char),
        }
    }
    out
}

/// The inverse of [`file_uri`]: the path a `file://` URI names, or `None`
/// for any other scheme — the compiler carries its standard library inside
/// the executable as `bundled:///...`, and there is no document behind such
/// a URI for an editor to open.
pub(crate) fn uri_path(uri: &str) -> Option<PathBuf> {
    let rest = uri.strip_prefix("file://")?;
    let mut out = Vec::with_capacity(rest.len());
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(byte) = u8::from_str_radix(&rest[i + 1..i + 3], 16)
        {
            out.push(byte);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    Some(PathBuf::from(String::from_utf8_lossy(&out).into_owned()))
}

/// Where the `tsgo` executable that serves the LSP comes from: the one
/// resolution both halves of the TypeScript toolchain share
/// ([`super::toolchain`]). Re-exported here because this module is what the
/// engine asks for a language server.
pub(crate) use super::toolchain::service_binary;

#[cfg(test)]
mod tests {
    use std::sync::mpsc::channel;
    use std::time::Duration;

    use super::{Arrangement, Response, ResponseFailure, language_id, wait_for_response};

    #[test]
    fn a_non_ascii_path_is_percent_encoded_as_utf8_and_read_back() {
        let path = std::path::Path::new("/p/ö 日本/aé.tt");
        let uri = super::file_uri(path);
        assert_eq!(uri, "file:///p/%C3%B6%20%E6%97%A5%E6%9C%AC/a%C3%A9.tt");
        assert_eq!(super::uri_path(&uri).as_deref(), Some(path));
    }

    #[test]
    fn projected_ttx_documents_open_as_typescript_react() {
        assert_eq!(
            language_id("file:///project/view.ttx.tsx"),
            "typescriptreact"
        );
        assert_eq!(language_id("file:///project/model.tt.ts"), "typescript");
        assert_eq!(language_id("file:///project/view.ttx"), "typescriptreact");
        assert_eq!(language_id("file:///project/model.tt"), "typescript");
    }

    fn mapper_project(manifest: Option<serde_json::Value>) -> crate::test_workspace::Workspace {
        let dir = crate::test_workspace::Workspace::with_subdir("service-arrangement", "app");
        if let Some(manifest) = manifest {
            let package = dir.join("node_modules/@openload28/tt-lang");
            std::fs::create_dir_all(&package).unwrap();
            std::fs::write(package.join("package.json"), manifest.to_string()).unwrap();
        }
        dir
    }

    fn tt_lang(extensions: serde_json::Value) -> serde_json::Value {
        serde_json::json!({ "package": "@openload28/tt-lang", "extensions": extensions })
    }

    #[test]
    fn only_a_configuration_naming_the_installed_tt_mapper_alone_serves_sources() {
        let installed = serde_json::json!({
            "name": "@openload28/tt-lang",
            "version": "1.2.3",
            "typescript": { "contentMapper": { "exec": ["node", "bin/ttc.js", "--content-mapper"] } },
        });
        let dir = mapper_project(Some(installed));
        let config = dir.join("app/tsconfig.json");
        let package = std::fs::canonicalize(dir.join("node_modules/@openload28/tt-lang")).unwrap();

        let mapped =
            Arrangement::of_configuration(&[tt_lang(serde_json::json!([".tt", ".ttx"]))], &config);
        assert_eq!(
            mapped.inferred_mapper,
            Some(serde_json::json!({
                "contributorId": "tt",
                "extensions": [".tt", ".ttx"],
                "inferredProjectContribution": {
                    "manifest": {
                        "name": "@openload28/tt-lang",
                        "version": "1.2.3",
                        "exec": ["node", "bin/ttc.js", "--content-mapper"],
                        "cwd": package,
                    },
                },
            }))
        );
        assert!(
            Arrangement::of_configuration(&[tt_lang(serde_json::json!([".tt"]))], &config)
                .inferred_mapper
                .is_some()
        );

        for configured in [
            vec![],
            vec![tt_lang(serde_json::json!([]))],
            vec![tt_lang(serde_json::json!([".tt", ".foo"]))],
            vec![
                tt_lang(serde_json::json!([".tt", ".ttx"])),
                serde_json::json!({ "package": "foo-mapper", "extensions": [".foo"] }),
            ],
            vec![serde_json::json!({ "package": "other-tt", "extensions": [".tt"] })],
        ] {
            assert_eq!(
                Arrangement::of_configuration(&configured, &config),
                Arrangement::default(),
                "{configured:?}"
            );
        }

        for manifest in [
            None,
            Some(serde_json::json!({ "name": "@openload28/tt-lang" })),
            Some(serde_json::json!({ "typescript": { "contentMapper": { "exec": [] } } })),
            Some(serde_json::json!({ "typescript": { "contentMapper": { "exec": ["node", 1] } } })),
        ] {
            let dir = mapper_project(manifest.clone());
            assert_eq!(
                Arrangement::of_configuration(
                    &[tt_lang(serde_json::json!([".tt", ".ttx"]))],
                    &dir.join("app/tsconfig.json"),
                ),
                Arrangement::default(),
                "{manifest:?}"
            );
        }
    }

    #[test]
    fn a_protocol_error_is_not_an_empty_type_answer() {
        let (tx, rx) = channel();
        tx.send(Response {
            id: 7,
            result: serde_json::Value::Null,
            error: Some("project graph could not be loaded".to_string()),
        })
        .expect("response receiver is alive");

        let error = wait_for_response(&rx, 7, "textDocument/hover", Duration::from_secs(1))
            .expect_err("the protocol error must remain an error");
        let ResponseFailure::Protocol(error) = error else {
            panic!("the service is still connected");
        };
        assert_eq!(error, "project graph could not be loaded");
    }

    #[test]
    fn a_timeout_is_not_an_empty_type_answer() {
        let (_tx, rx) = channel();
        let error = wait_for_response(&rx, 11, "textDocument/completion", Duration::from_millis(1))
            .expect_err("the timeout must remain an error");
        let ResponseFailure::Timeout(error) = error else {
            panic!("the service is still connected");
        };
        assert_eq!(
            error,
            "TypeScript language service request `textDocument/completion` timed out after 1ms"
        );
    }

    #[test]
    fn a_successful_null_remains_an_empty_type_answer() {
        let (tx, rx) = channel();
        tx.send(Response {
            id: 13,
            result: serde_json::Value::Null,
            error: None,
        })
        .expect("response receiver is alive");

        assert_eq!(
            wait_for_response(&rx, 13, "textDocument/hover", Duration::from_secs(1))
                .expect("null is a successful answer"),
            serde_json::Value::Null
        );
    }
}
