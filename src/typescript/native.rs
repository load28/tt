//! The native TypeScript backend — the only module that knows how the
//! compiler is reached.
//!
//! The compiler is `tsgo`, the native TypeScript 7 compiler, driven through
//! its API server. ttc talks to that server the way the TypeScript team's own
//! client does: a small host process ([`HOST`]) running under `node`, which
//! imports the JS client and speaks the server's MessagePack protocol.
//!
//! Client and server are **one unit**: the protocol carries no version
//! negotiation and the client is generated from the server's Go source, so
//! both come from one install. *Which* install is [`super::toolchain`]'s
//! answer — the language service asks it the same question for its own half.
//! Everything unstable about TypeScript 7 lives behind this module and
//! [`super::backend::TypeScriptBackend`].

use std::cell::RefCell;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};

use super::backend::*;
use super::toolchain::{self, Client};

/// The host script, embedded so a released `ttc` needs no files beside it.
const HOST: &str = include_str!("host.mjs");

/// A [`TypeScriptBackend`] over a running compiler.
///
/// The first question starts the host and opens the project; every question
/// after it reuses both. That is the difference between a watch that
/// re-checks in milliseconds and one that pays for a whole project open per
/// keystroke.
#[derive(Debug)]
pub(crate) struct NativeBackend {
    toolchain: Client,
    /// The `node` binary that runs the host (`--node`, else `node` on PATH).
    node: PathBuf,
    session: RefCell<Option<Session>>,
}

/// The running host: a process, and the two pipes a request travels over.
#[derive(Debug)]
struct Session {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    /// What the project was opened as. A question about a different project
    /// needs a different session.
    opened: (Option<PathBuf>, PathBuf),
    /// The host script, kept until the session ends.
    script: PathBuf,
}

impl NativeBackend {
    /// Resolves the toolchain and prepares a backend over it. Nothing is
    /// started until the first question.
    pub(crate) fn new(node: Option<PathBuf>, from: &Path) -> Result<NativeBackend, String> {
        Ok(NativeBackend {
            toolchain: toolchain::client(from)?,
            node: node.unwrap_or_else(|| PathBuf::from("node")),
            session: RefCell::new(None),
        })
    }

    /// Starts the host and opens the project.
    fn start(&self, tsconfig: Option<&Path>, root: &Path) -> Result<Session, Failure> {
        // The host is written beside the run rather than piped in: node reads
        // a module from a path, and the path is what import specifiers in the
        // job resolve against.
        let dir = std::env::temp_dir().join(format!("ttc-host-{}", std::process::id()));
        std::fs::create_dir_all(&dir)
            .map_err(|e| Failure::unavailable(format!("cannot prepare the host: {e}")))?;
        let script = dir.join("host.mjs");
        std::fs::write(&script, HOST)
            .map_err(|e| Failure::unavailable(format!("cannot write the host: {e}")))?;

        let mut child = Command::new(&self.node)
            .arg(&script)
            .current_dir(root)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| {
                Failure::unavailable(format!("cannot run {}: {e}", self.node.display()))
            })?;
        let mut stdin = child.stdin.take().expect("stdin piped");
        let mut stdout = BufReader::new(child.stdout.take().expect("stdout piped"));

        let open = serde_json::json!({
            "apiModule": self.toolchain.api,
            "cwd": root,
            "tsconfig": tsconfig,
        });
        writeln!(stdin, "{open}")
            .map_err(|e| Failure::unavailable(format!("cannot start the host: {e}")))?;

        let mut ack = String::new();
        if stdout
            .read_line(&mut ack)
            .map_err(|e| Failure::unavailable(e.to_string()))?
            == 0
        {
            return Err(host_died(&mut child));
        }
        Ok(Session {
            child,
            stdin,
            stdout,
            opened: (tsconfig.map(Path::to_path_buf), root.to_path_buf()),
            script,
        })
    }
}

/// What the host said on its way out. A crash before the first answer is
/// usually a missing API or an unreadable client, and its message is on
/// stderr.
fn host_died(child: &mut Child) -> Failure {
    let status = child.wait().ok();
    let mut stderr = String::new();
    if let Some(mut pipe) = child.stderr.take() {
        use std::io::Read;
        let _ = pipe.read_to_string(&mut stderr);
    }
    if status.and_then(|s| s.code()) == Some(5) {
        return Failure::unavailable(
            "the installed TypeScript can check but cannot emit \
                declarations — that API arrived in TypeScript 7.1. Install a \
                7.1 in this project (`npm i -D typescript@7.1`), or use \
                --check-types, which writes nothing",
        );
    }
    let stderr = stderr.trim();
    let message = format!(
        "the TypeScript backend failed:\n{}",
        if stderr.is_empty() {
            "(no output)"
        } else {
            stderr
        }
    );
    if status.and_then(|s| s.code()) == Some(2) {
        Failure::unavailable(message)
    } else {
        Failure::internal(message)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        // Closing stdin ends the host's loop; the wait keeps it from
        // outliving the run.
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.script);
    }
}

impl TypeScriptBackend for NativeBackend {
    fn ask(&self, tsconfig: Option<&Path>, root: &Path, query: &Query) -> Result<Answers, Failure> {
        let wanted = (tsconfig.map(Path::to_path_buf), root.to_path_buf());
        let mut slot = self.session.borrow_mut();
        // A question about a different project needs its own session: the
        // project is opened once and never reopened.
        if slot.as_ref().is_some_and(|s| s.opened != wanted) {
            *slot = None;
        }
        if slot.is_none() {
            *slot = Some(self.start(tsconfig, root)?);
        }
        let session = slot.as_mut().expect("started");

        let job = job_json(query);
        let answer = exchange(session, &job.to_string());
        match answer {
            Ok(line) => parse_answers(&line),
            Err(_) => {
                // The host is gone; take its last words, and let the next
                // question start a fresh one.
                let mut session = slot.take().expect("started");
                Err(host_died(&mut session.child))
            }
        }
    }
}

/// One request, one answer. An I/O error means the host is no longer there.
fn exchange(session: &mut Session, request: &str) -> std::io::Result<String> {
    writeln!(session.stdin, "{request}")?;
    session.stdin.flush()?;
    let mut line = String::new();
    if session.stdout.read_line(&mut line)? == 0 {
        return Err(std::io::Error::from(std::io::ErrorKind::UnexpectedEof));
    }
    Ok(line)
}

/// One `ask` — see `host.mjs` for the protocol.
fn job_json(query: &Query) -> serde_json::Value {
    use serde_json::json;
    json!({
        "modules": query.modules.iter()
            .map(|m| json!({ "path": m.path, "text": m.text }))
            .collect::<Vec<_>>(),
        "sources": query.sources,
        "literalChecks": query.literals.iter()
            .map(|l| json!({
                "module": l.module,
                "start": l.position,
                "covered": l.covered.iter().map(literal_json).collect::<Vec<_>>(),
            }))
            .collect::<Vec<_>>(),
        "tagChecks": query.tags.iter()
            .map(|t| json!({
                "module": t.module,
                "start": t.position,
                "covered": t.covered,
            }))
            .collect::<Vec<_>>(),
        "symbolChecks": query.symbols.iter()
            .map(|v| json!({ "module": v.module, "start": v.position }))
            .collect::<Vec<_>>(),
        "resultShapeChecks": query.result_shapes.iter()
            .map(|v| json!({ "module": v.module, "start": v.position }))
            .collect::<Vec<_>>(),
        "emitDeclarations": query.emit_declarations,
    })
}

/// A covered literal as the value JavaScript compares with `===`.
fn literal_json(literal: &crate::Literal) -> serde_json::Value {
    use serde_json::json;
    match literal {
        crate::Literal::String(s) => json!(s),
        crate::Literal::Number(n) => json!(n),
        crate::Literal::Boolean(b) => json!(b),
        // No finite literal union TypeScript reports holds a BigInt, so a
        // match covering one is never asked about; carried as text for
        // completeness.
        crate::Literal::BigInt(d) => json!(d),
    }
}

/// Reads the host's answer. A shape that does not match is a bug in the pair
/// of this file and `host.mjs`, and is reported as one.
fn parse_answers(stdout: &str) -> Result<Answers, Failure> {
    let value: serde_json::Value = serde_json::from_str(stdout.trim()).map_err(|e| {
        Failure::internal(format!(
            "the TypeScript backend answered with malformed JSON: {e}"
        ))
    })?;
    if let Some(error) = value["error"].as_str() {
        return Err(Failure::internal(format!(
            "the TypeScript backend failed:\n{error}"
        )));
    }

    let mut answers = Answers::default();
    let project_modules = value["projectModules"]
        .as_array()
        .ok_or_else(|| Failure::internal("the TypeScript backend answer omitted projectModules"))?;
    answers.project_modules = Some(
        project_modules
            .iter()
            .filter_map(|module| module.as_str().map(PathBuf::from))
            .collect(),
    );
    for d in array(&value, "diagnostics") {
        answers.diagnostics.push(Diagnostic {
            file: PathBuf::from(d["file"].as_str().unwrap_or_default()),
            start: d["start"].as_u64().unwrap_or_default() as usize,
            end: d["end"].as_u64().unwrap_or_default() as usize,
            code: d["code"].as_u64().unwrap_or_default() as u32,
            message: d["message"].as_str().unwrap_or_default().to_string(),
            mismatch: parse_type_mismatch(&d["mismatch"]),
            related: d["related"]
                .as_array()
                .map(|entries| {
                    entries
                        .iter()
                        .filter_map(|r| {
                            Some(RelatedInformation {
                                file: PathBuf::from(r["file"].as_str()?),
                                start: r["start"].as_u64()? as usize,
                                end: r["end"].as_u64()? as usize,
                                message: r["message"].as_str()?.to_string(),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    for m in array(&value, "literalMissing") {
        answers.literal_missing.push(LiteralMissing {
            index: m["index"].as_u64().unwrap_or_default() as usize,
            missing: m["missing"]
                .as_array()
                .map(|a| a.iter().filter_map(json_literal).collect())
                .unwrap_or_default(),
        });
    }
    for m in array(&value, "tagMissing") {
        answers.tag_missing.push(TagMissing {
            index: m["index"].as_u64().unwrap_or_default() as usize,
            missing: m["missing"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    for m in array(&value, "tagMembers") {
        answers.tag_members.push(TagMembers {
            index: m["index"].as_u64().unwrap_or_default() as usize,
            tags: m["tags"]
                .as_array()
                .map(|a| {
                    a.iter()
                        .filter_map(|s| s.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default(),
        });
    }
    for v in array(&value, "symbols") {
        answers.resolutions.push(Resolution {
            index: v["index"].as_u64().unwrap_or_default() as usize,
            id: v["id"].as_i64().unwrap_or_default(),
            name: v["name"].as_str().unwrap_or_default().to_string(),
            builtin: v["builtin"].as_bool().unwrap_or(false),
        });
    }
    for v in array(&value, "resultShapes") {
        answers.result_shapes.push(ResultShape {
            index: v["index"].as_u64().unwrap_or_default() as usize,
        });
    }
    for d in array(&value, "declarations") {
        answers.declarations.push(Declaration {
            path: PathBuf::from(d["path"].as_str().unwrap_or_default()),
            text: d["text"].as_str().unwrap_or_default().to_string(),
        });
    }
    Ok(answers)
}

fn parse_type_mismatch(value: &serde_json::Value) -> Option<TypeMismatch> {
    let object = value.as_object()?;
    let differences = object
        .get("differences")?
        .as_array()?
        .iter()
        .filter_map(|difference| {
            Some(TypeDifference {
                expected: difference["expected"].as_str()?.to_string(),
                found: difference["found"].as_str()?.to_string(),
            })
        })
        .collect();
    Some(TypeMismatch {
        start: object.get("start")?.as_u64()? as usize,
        end: object.get("end")?.as_u64()? as usize,
        expected: object.get("expected")?.as_str()?.to_string(),
        found: object.get("found")?.as_str()?.to_string(),
        differences,
    })
}

fn array<'a>(value: &'a serde_json::Value, key: &str) -> &'a [serde_json::Value] {
    value[key].as_array().map_or(&[], |a| a.as_slice())
}

/// A literal the checker reported, in tt's own vocabulary.
fn json_literal(value: &serde_json::Value) -> Option<crate::Literal> {
    match value {
        serde_json::Value::String(s) => Some(crate::Literal::String(s.clone())),
        serde_json::Value::Number(n) => n.as_f64().map(crate::Literal::Number),
        serde_json::Value::Bool(b) => Some(crate::Literal::Boolean(*b)),
        _ => None,
    }
}
