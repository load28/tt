//! The public surface as baselines: the library's API as rustdoc renders
//! it, the `ttc --server` protocol, and the capabilities the VS Code
//! extension's language server advertises.
//!
//! TypeScript holds its API to `tests/baselines/reference/api/typescript.d.ts`
//! (`src/testRunner/unittests/publicApi.ts`, "should be acknowledged when
//! they change"): the compiler's own declaration output, compared as a
//! baseline, so an API change is a reviewed diff. The same applies here to
//! each surface a consumer depends on.

mod common;

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use common::Workspace;
use common::baseline::expect;
use common::cases::DEFAULT_TSCONFIG;
use serde_json::{Value, json};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn reference() -> PathBuf {
    root().join("tests/baselines/reference/api")
}

fn decode(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(open) = rest.find('<') {
        text.push_str(&rest[..open]);
        let close = rest[open..].find('>').map_or(rest.len(), |i| open + i + 1);
        rest = &rest[close..];
    }
    text.push_str(rest);
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
}

fn without_anchors(html: &str) -> String {
    let mut out = String::with_capacity(html.len());
    let mut rest = html;
    while let Some(start) = rest.find("<a href=\"#") {
        let tag_end = rest[start..].find('>').map_or(rest.len(), |i| start + i);
        if !rest[start..tag_end].contains("class=\"anchor") {
            out.push_str(&rest[..tag_end]);
            rest = &rest[tag_end..];
            continue;
        }
        out.push_str(&rest[..start]);
        let end = rest[start..]
            .find("</a>")
            .map_or(rest.len(), |i| start + i + 4);
        rest = &rest[end..];
    }
    out.push_str(rest);
    out
}

fn between<'a>(text: &'a str, open: &str, close: &str) -> Vec<&'a str> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        let body = &rest[start + open.len()..];
        let Some(end) = body.find(close) else {
            break;
        };
        out.push(&body[..end]);
        rest = &body[end + close.len()..];
    }
    out
}

fn section_of(page: &str, at: usize) -> &str {
    let before = &page[..at];
    let Some(heading) = before.rfind("<h2 id=\"") else {
        return "";
    };
    let id = &before[heading + 8..];
    &id[..id.find('"').unwrap_or(0)]
}

fn item_listing(page: &str) -> String {
    let page = without_anchors(page);
    let mut out = String::new();
    for decl in between(
        &page,
        "<pre class=\"rust item-decl\"><code>",
        "</code></pre>",
    ) {
        out.push_str(&decode(decl));
        out.push('\n');
    }
    let mut rest = page.as_str();
    let mut offset = 0;
    while let Some(start) = rest.find("class=\"code-header\">") {
        let level = &rest[..start];
        let level = &level[level.rfind('<').map_or(0, |i| i + 1)..];
        let body = &rest[start + 20..];
        let close = body.find("</h").unwrap_or(body.len());
        let header = decode(&body[..close]);
        let section = section_of(&page, offset + start);
        let keep = match section {
            "implementations" => true,
            "trait-implementations" | "synthetic-implementations" | "implementors" => {
                level.starts_with("h3")
            }
            "required-methods"
            | "provided-methods"
            | "required-associated-types"
            | "provided-associated-types"
            | "required-associated-consts"
            | "provided-associated-consts" => true,
            _ => false,
        };
        if keep {
            let indent = if level.starts_with("h3") { "" } else { "    " };
            for line in header.lines() {
                out.push_str(indent);
                out.push_str(line);
                out.push('\n');
            }
        }
        offset += start + 20 + close;
        rest = &body[close..];
    }
    out
}

fn rust_api() -> String {
    let target = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .parent()
        .expect("the target directory")
        .to_path_buf();
    let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let output = Command::new(cargo)
        .args(["doc", "--no-deps", "--lib", "--locked", "--target-dir"])
        .arg(&target)
        .current_dir(root())
        .env_remove("RUSTDOCFLAGS")
        .output()
        .expect("cargo doc runs");
    assert!(
        output.status.success(),
        "cargo doc failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let docs = target.join("doc/ttc");
    let all = fs::read_to_string(docs.join("all.html")).expect("rustdoc's list of all items");
    let mut pages: BTreeSet<String> = BTreeSet::new();
    for list in between(&all, "<ul class=\"all-items\">", "</ul>") {
        for href in between(list, "<a href=\"", "\"") {
            pages.insert(href.to_string());
        }
    }
    assert!(!pages.is_empty(), "rustdoc listed no public item");
    let mut out = String::new();
    for href in pages {
        let page = fs::read_to_string(docs.join(&href)).expect("an item page");
        let (dir, file) = href.rsplit_once('/').unwrap_or(("", href.as_str()));
        let (kind, name) = file
            .trim_end_matches(".html")
            .split_once('.')
            .expect("an item page is named kind.name.html");
        let path = if dir.is_empty() {
            format!("ttc::{name}")
        } else {
            format!("ttc::{}::{name}", dir.replace('/', "::"))
        };
        out.push_str(&format!("// {kind} {path}\n{}\n", item_listing(&page)));
    }
    out
}

#[test]
fn the_rust_api_matches_its_baseline() {
    expect(&reference().join("ttc.api.txt"), &rust_api());
}

const MAIN: &str = r#"import { helper } from "./helper.tt";

export variant Shape {
  Circle(radius: number),
  Square(side: number),
}

/** The area of a shape. */
export function area(shape: Shape): number {
  return match (shape) {
    Circle(radius) => Math.PI * radius * radius,
    Square(side) => side * side,
  };
}

export const total = area(Shape.Circle(helper(2)));
console.log(total.toFixed(1));
const label = Shape.Square(3);
const broken: string = 1;
"#;

const HELPER: &str = "export function helper(n: number): number {\n  return n * 2;\n}\n";

fn at(needle: &str, delta: usize) -> Value {
    let offset = MAIN.find(needle).expect("a needle in MAIN") + delta;
    let before = &MAIN[..offset];
    let line = before.matches('\n').count();
    let character = before.len() - before.rfind('\n').map_or(0, |i| i + 1);
    json!({ "line": line, "character": character })
}

fn examples(path: &str) -> Vec<(&'static str, Value)> {
    vec![
        (
            "check",
            json!({ "text": MAIN, "filename": "main.tt", "verify": true }),
        ),
        (
            "check",
            json!({ "text": "variant V { A, A }\nconst v = match (V.A) { A => 1 };\n", "filename": "bad.tt" }),
        ),
        ("emitMap", json!({ "text": MAIN, "filename": "main.tt" })),
        ("semanticTokens", json!({ "text": MAIN })),
        ("openDocument", json!({ "path": path, "text": MAIN })),
        ("updateDocument", json!({ "path": path, "text": MAIN })),
        (
            "typedCheck",
            json!({ "path": path, "text": MAIN, "includeTypes": true }),
        ),
        (
            "print",
            json!({ "path": path, "sourceMap": "inline", "rewriteImports": "ts", "banner": false, "verify": true }),
        ),
        ("dependencies", json!({ "path": path })),
        (
            "hover",
            json!({ "path": path, "position": at("area(Shape", 1) }),
        ),
        (
            "hover",
            json!({ "path": path, "position": at("\n\nexport variant", 1) }),
        ),
        (
            "definition",
            json!({ "path": path, "position": at("helper(2)", 1) }),
        ),
        (
            "references",
            json!({ "path": path, "position": at("area(shape", 1) }),
        ),
        (
            "completion",
            json!({ "path": path, "position": at("total.toFixed", 6), "member": true, "triggerCharacter": "." }),
        ),
        (
            "completionResolve",
            json!({ "path": path, "position": at("total.toFixed", 6), "label": "toFixed", "source": null, "probe": "$probe" }),
        ),
        (
            "prepareRename",
            json!({ "path": path, "position": at("total =", 1) }),
        ),
        (
            "rename",
            json!({ "path": path, "position": at("total =", 1) }),
        ),
        ("documentSymbols", json!({ "path": path })),
        (
            "signatureHelp",
            json!({ "path": path, "position": at("area(Shape", 5), "triggerKind": 2, "triggerCharacter": "(", "isRetrigger": false }),
        ),
        (
            "patternCompletions",
            json!({ "path": path, "position": at("Circle(radius) =>", 0) }),
        ),
        ("documentSemanticTokens", json!({ "path": path })),
        ("tsDiagnostics", json!({ "path": path })),
        ("declarations", json!({ "path": path, "text": MAIN })),
        (
            "ttSymbol",
            json!({ "path": path, "text": MAIN, "position": at("Circle(radius: number)", 1) }),
        ),
        (
            "ttCompletions",
            json!({ "path": path, "text": MAIN, "position": at("Shape.Square(3)", 6) }),
        ),
        ("ttHints", json!({ "path": path, "text": MAIN })),
        ("closeDocument", json!({ "path": path })),
        ("reloadProjects", json!({})),
        ("noSuchMethod", json!({})),
    ]
}

fn kind(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[derive(Default)]
struct Shape {
    scalars: BTreeSet<&'static str>,
    elements: Option<Box<Shape>>,
    empty_array: bool,
    fields: Option<BTreeMap<String, (Shape, usize)>>,
    objects: usize,
}

impl Shape {
    fn add(&mut self, value: &Value) {
        match value {
            Value::Array(items) => {
                if items.is_empty() {
                    self.empty_array = true;
                }
                let elements = self.elements.get_or_insert_with(Default::default);
                for item in items {
                    elements.add(item);
                }
            }
            Value::Object(map) => {
                self.objects += 1;
                let fields = self.fields.get_or_insert_with(Default::default);
                for (key, field) in map {
                    let entry = fields.entry(key.clone()).or_default();
                    entry.0.add(field);
                    entry.1 += 1;
                }
            }
            scalar => {
                self.scalars.insert(kind(scalar));
            }
        }
    }

    fn render(&self, indent: usize) -> String {
        let mut alternatives: Vec<String> = Vec::new();
        if self.fields.as_ref().is_some_and(|fields| fields.is_empty()) {
            alternatives.push("{}".to_string());
        } else if let Some(fields) = &self.fields {
            let pad = "  ".repeat(indent + 1);
            let mut text = String::from("{\n");
            for (key, (shape, seen)) in fields {
                let optional = if *seen < self.objects { "?" } else { "" };
                text.push_str(&format!(
                    "{pad}{key}{optional}: {}\n",
                    shape.render(indent + 1)
                ));
            }
            text.push_str(&"  ".repeat(indent));
            text.push('}');
            alternatives.push(text);
        }
        match &self.elements {
            Some(elements)
                if elements.fields.is_some()
                    || elements.elements.is_some()
                    || !elements.scalars.is_empty() =>
            {
                alternatives.push(format!("[{}]", elements.render(indent)));
            }
            Some(_) => alternatives.push("[]".to_string()),
            None => {}
        }
        alternatives.extend(
            self.scalars
                .iter()
                .filter(|scalar| **scalar != "null")
                .map(|s| s.to_string()),
        );
        if self.scalars.contains("null") {
            alternatives.push("null".to_string());
        }
        alternatives.join(" | ")
    }
}

fn shape(value: &Value) -> String {
    let mut shape = Shape::default();
    shape.add(value);
    shape.render(1)
}

fn dispatched_methods() -> BTreeSet<String> {
    let source = fs::read_to_string(root().join("src/server.rs")).expect("the server source");
    let respond = &source[source.find("fn respond(").expect("the dispatcher")..];
    let respond = &respond[..respond
        .find("method => Err(format!(\"unknown method")
        .expect("the dispatcher's fallback")];
    let mut methods = BTreeSet::new();
    for line in respond.lines() {
        let line = line.trim_start();
        if !line.starts_with('"') || !line.contains("=>") {
            continue;
        }
        let pattern = &line[..line.find("=>").unwrap()];
        for name in pattern.split('|') {
            methods.insert(name.trim().trim_matches('"').to_string());
        }
    }
    methods
}

fn read_params() -> BTreeSet<String> {
    let source = fs::read_to_string(root().join("src/server.rs")).expect("the server source");
    between(&source, "params[\"", "\"]")
        .into_iter()
        .map(str::to_string)
        .collect()
}

fn protocol() -> String {
    let workspace = Workspace::in_repo("protocol");
    let dir = workspace.path().canonicalize().expect("a workspace");
    fs::write(dir.join("main.tt"), MAIN).expect("writable");
    fs::write(dir.join("helper.tt"), HELPER).expect("writable");
    fs::write(dir.join("tsconfig.json"), DEFAULT_TSCONFIG).expect("writable");
    let path = dir.join("main.tt").to_string_lossy().into_owned();
    let examples = examples(&path);

    let dispatched = dispatched_methods();
    let exemplified: BTreeSet<String> = examples
        .iter()
        .map(|(method, _)| method.to_string())
        .filter(|method| method != "noSuchMethod")
        .collect();
    assert_eq!(
        dispatched, exemplified,
        "every method src/server.rs dispatches needs an example request in tests/public_api.rs, and every example a dispatched method"
    );
    let named: BTreeSet<String> = examples
        .iter()
        .filter_map(|(_, params)| params.as_object())
        .flat_map(|params| params.keys().cloned())
        .collect();
    let unexemplified: Vec<_> = read_params().difference(&named).cloned().collect();
    assert!(
        unexemplified.is_empty(),
        "src/server.rs reads params no example request sends: {unexemplified:?}"
    );

    let mut child = Command::new(env!("CARGO_BIN_EXE_ttc"))
        .arg("--server")
        .current_dir(&dir)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("ttc --server starts");
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let mut out = format!(
        "methods: {}\nparams read: {}\n",
        dispatched.into_iter().collect::<Vec<_>>().join(", "),
        read_params().into_iter().collect::<Vec<_>>().join(", ")
    );
    let mut probe = Value::Null;
    for (id, (method, mut params)) in examples.into_iter().enumerate() {
        if params["probe"] == json!("$probe") {
            params["probe"] = probe.clone();
        }
        let request = json!({ "id": id, "method": method, "params": params });
        writeln!(stdin, "{request}").expect("the server reads");
        stdin.flush().expect("the server reads");
        let mut line = String::new();
        stdout.read_line(&mut line).expect("the server answers");
        let answer: Value = serde_json::from_str(&line).expect("a JSON answer");
        assert_eq!(answer["id"], json!(id), "answers come in order");
        if method == "completion" {
            probe = answer["result"]["probe"].clone();
        }
        out.push_str(&format!(
            "\n→ {method} {}\n← {}\n",
            shape(&params),
            shape(&answer)
        ));
    }
    drop(stdin);
    let _ = child.wait();
    out
}

#[test]
fn the_server_protocol_matches_its_baseline() {
    if !common::toolchain() {
        eprintln!("SKIP the server protocol baseline: no TypeScript installed — run `npm ci`");
        return;
    }
    expect(&reference().join("server-protocol.txt"), &protocol());
}

fn extension_server() -> Option<PathBuf> {
    let server = root().join("editors/vscode/server/out/server.js");
    let dependency = root().join("editors/vscode/server/node_modules/vscode-languageserver");
    if server.is_file() && dependency.is_dir() {
        return Some(server);
    }
    assert!(
        std::env::var_os("TT_REQUIRE_EXTENSION").is_none_or(|v| v.is_empty() || v == "0"),
        "TT_REQUIRE_EXTENSION is set but the extension's language server is not built — \
         run `npm ci --prefix editors/vscode && npm --prefix editors/vscode run compile`"
    );
    None
}

fn lsp_message(stdout: &mut impl BufRead) -> Value {
    let mut length = 0usize;
    loop {
        let mut header = String::new();
        stdout.read_line(&mut header).expect("an LSP header");
        let header = header.trim_end();
        if header.is_empty() {
            break;
        }
        if let Some(value) = header.strip_prefix("Content-Length:") {
            length = value.trim().parse().expect("a content length");
        }
    }
    let mut body = vec![0; length];
    stdout.read_exact(&mut body).expect("an LSP body");
    serde_json::from_slice(&body).expect("a JSON body")
}

fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let ordered: BTreeMap<_, _> = map.iter().map(|(k, v)| (k.clone(), sorted(v))).collect();
            json!(ordered)
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

#[test]
fn the_extension_capabilities_match_their_baseline() {
    let Some(server) = extension_server() else {
        eprintln!(
            "SKIP the extension capabilities baseline: its language server is not built \
             (npm ci --prefix editors/vscode && npm --prefix editors/vscode run compile)"
        );
        return;
    };
    let workspace = Workspace::in_repo("capabilities");
    let mut child = Command::new("node")
        .arg(&server)
        .arg("--stdio")
        .current_dir(workspace.path())
        .env("TTC_BINARY", env!("CARGO_BIN_EXE_ttc"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("node runs the extension's language server");
    let mut stdin = child.stdin.take().expect("piped stdin");
    let mut stdout = BufReader::new(child.stdout.take().expect("piped stdout"));
    let body = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "initialize",
        "params": { "processId": null, "rootUri": null, "capabilities": {} },
    })
    .to_string();
    write!(stdin, "Content-Length: {}\r\n\r\n{body}", body.len()).expect("the server reads");
    stdin.flush().expect("the server reads");
    let deadline = Instant::now() + Duration::from_secs(60);
    let result = loop {
        assert!(Instant::now() < deadline, "no initialize answer");
        let message = lsp_message(&mut stdout);
        if message["id"] == json!(1) {
            break message["result"].clone();
        }
    };
    let _ = child.kill();
    let _ = child.wait();
    let text = serde_json::to_string_pretty(&sorted(&result)).expect("serializable");
    expect(
        &reference().join("lsp-capabilities.json"),
        &format!("{text}\n"),
    );
}
