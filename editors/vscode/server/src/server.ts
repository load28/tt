/* --------------------------------------------------------------------------
 * tt language server — a protocol adapter over the tt engine.
 *
 * The server owns the LSP surface: capabilities, document sync with the
 * client, debouncing, configuration, and presentation. Language semantics
 * live elsewhere and are consumed, not implemented, here:
 *
 * - The **engine** (`ttc --server`, engine.ts) is the authoritative project
 *   state. The editor's buffers are forwarded to it (didOpen/didChange/
 *   didClose), and every TypeScript-backed answer — hover, definition,
 *   references, completion with its incomplete-source probe, rename,
 *   signature help, type diagnostics — comes back from it already in `.tt`
 *   coordinates. No projection, source mapping, TypeScript session or
 *   probe logic lives in this process.
 * - **tt's own names** — variant names, case tags, payload fields — come from
 *   the engine too (`ttSymbol`, `ttCompletions`), even though no type is
 *   involved: they exist nowhere in the emitted TypeScript, so the
 *   compiler's declaration table is the only thing that knows them, and a
 *   second opinion here would answer differently from the compiler
 *   (TASK-107). Those requests are text-based, so they work on a buffer
 *   mid-keystroke and with no TypeScript toolchain installed.
 * - The **declaration surface** — which variants are visible (local,
 *   imported, built-in), their cases and fields, and where the `match`
 *   sites are — is the compiler's answer too (`declarations`,
 *   `declarationsOf`): completion lists, document symbols and the
 *   missing-arms quick fix all read it, so this process holds no second
 *   implementation of the compiler's rules (TASK-128; the old regex layer
 *   could disagree with the compiler and is gone).
 * - The **text-shape layer** (analysis.ts) is what is left after that:
 *   cursor context over raw text — whether a `.` is a member access,
 *   the word under the cursor, masking. It is deliberately
 *   diagnostic-free — a near-miss can lose a convenience, never invent an
 *   error.
 * - Diagnostics run the real compiler through ttc.ts (`--check`, and the
 *   typed layer via the engine's `typedCheck`).
 * ----------------------------------------------------------------------- */
import {
  CodeAction,
  CodeActionKind,
  CompletionItem,
  CompletionItemKind,
  createConnection,
  Diagnostic,
  DiagnosticSeverity,
  DiagnosticTag,
  DocumentSymbol,
  InitializeParams,
  InitializeResult,
  InsertTextFormat,
  Location,
  MarkupKind,
  ParameterInformation,
  ProposedFeatures,
  Range,
  SemanticTokensBuilder,
  SignatureHelp,
  SignatureInformation,
  SymbolKind,
  TextDocuments,
  TextDocumentSyncKind,
  TextEdit,
} from "vscode-languageserver/node";
import { TextDocument } from "vscode-languageserver-textdocument";
import { URI } from "vscode-uri";

import * as analysis from "./analysis";
import * as engine from "./engine";
import * as ttc from "./ttc";
import * as path from "node:path";

import * as sidecar from "./sidecar";

const connection = createConnection(ProposedFeatures.all);
const documents = new TextDocuments(TextDocument);

let hasConfigurationCapability = false;
let workspaceRoots: string[] = [];
let warnedCompilerMissing = false;

connection.onInitialize((params: InitializeParams): InitializeResult => {
  hasConfigurationCapability = Boolean(
    params.capabilities.workspace?.configuration,
  );
  workspaceRoots = (params.workspaceFolders ?? [])
    .map((f) => URI.parse(f.uri))
    .filter((u) => u.scheme === "file")
    .map((u) => u.fsPath);

  return {
    capabilities: {
      textDocumentSync: {
        openClose: true,
        change: TextDocumentSyncKind.Incremental,
        // Saving is when the sidecars are rebuilt; the text is already on
        // disk by then, so the notification does not need to carry it.
        save: { includeText: false },
      },
      completionProvider: {
        triggerCharacters: [".", "(", "|"],
        // Signatures and documentation are fetched per entry, when the
        // editor asks for the one the user highlighted (onCompletionResolve).
        resolveProvider: true,
      },
      signatureHelpProvider: {
        triggerCharacters: ["(", ","],
        retriggerCharacters: [")"],
      },
      hoverProvider: true,
      definitionProvider: true,
      referencesProvider: true,
      renameProvider: true,
      documentSymbolProvider: true,
      codeActionProvider: { codeActionKinds: [CodeActionKind.QuickFix] },
      // The parser's own classification of the ambiguous surface (a `flow`
      // head split across lines, a plain function named `match`), layered
      // over the TextMate grammar per the LSP semantic-tokens contract.
      semanticTokensProvider: {
        legend: { tokenTypes: SEMANTIC_TOKEN_TYPES, tokenModifiers: [] },
        full: true,
        range: false,
      },
    },
  };
});

connection.onInitialized(() => {
  // The engine session is keyed by the compiler path, and the document
  // sync that starts it cannot await settings — so `tt.compilerPath` is
  // resolved once here, before the first buffer arrives.
  void refreshCompiler();
});

// ---------------------------------------------------------------- settings

interface TtSettings {
  compilerPath: string;
  verify: boolean;
  typeDiagnostics: boolean;
  typedChecks: boolean;
  sidecar: sidecar.SidecarMode;
  sidecarDir: string;
}

const DEFAULT_SETTINGS: TtSettings = {
  compilerPath: "",
  verify: true,
  typeDiagnostics: true,
  typedChecks: true,
  sidecar: "refresh",
  sidecarDir: "",
};

async function getSettings(uri?: string): Promise<TtSettings> {
  if (!hasConfigurationCapability) return DEFAULT_SETTINGS;
  const conf = (await connection.workspace.getConfiguration({
    scopeUri: uri,
    section: "tt",
  })) as Partial<TtSettings> | null;
  return {
    compilerPath: conf?.compilerPath ?? DEFAULT_SETTINGS.compilerPath,
    verify: conf?.verify ?? DEFAULT_SETTINGS.verify,
    typeDiagnostics: conf?.typeDiagnostics ?? DEFAULT_SETTINGS.typeDiagnostics,
    typedChecks: conf?.typedChecks ?? DEFAULT_SETTINGS.typedChecks,
    sidecar: conf?.sidecar ?? DEFAULT_SETTINGS.sidecar,
    sidecarDir: conf?.sidecarDir ?? DEFAULT_SETTINGS.sidecarDir,
  };
}

connection.onDidChangeConfiguration(() => {
  warnedCompilerMissing = false;
  // `tt.compilerPath` may be what changed, and a compiler that struck out
  // has earned another try either way.
  engine.retryEngineServer();
  void refreshCompiler();
  for (const doc of documents.all()) scheduleValidation(doc);
});

connection.onDidChangeWatchedFiles(() => {
  // A freshly built ttc appeared (or changed) — try validating again.
  warnedCompilerMissing = false;
  engine.retryEngineServer();
  void refreshCompiler();
  for (const doc of documents.all()) scheduleValidation(doc);
});

// ---------------------------------------------------------------- analysis

interface Analyzed {
  version: number;
  text: string;
  masked: string;
}

const analysisCache = new Map<string, Analyzed>();

/** The text-shape half: the masked buffer for cursor-context questions
 * (member access, word boundaries). tt *semantics* — which variants are
 * visible, where the match sites are — are the compiler's answer
 * ([declarationsOf]), not this file's. */
function analyze(doc: TextDocument): Analyzed {
  const cached = analysisCache.get(doc.uri);
  if (cached && cached.version === doc.version) return cached;
  const text = doc.getText();
  const masked = analysis.maskNonCode(text);
  const result: Analyzed = { version: doc.version, text, masked };
  analysisCache.set(doc.uri, result);
  return result;
}

const declCache = new Map<
  string,
  { version: number; decls: engine.EngineDeclarations }
>();

/** The compiler's declaration surface for the buffer: visible variants
 * (local, imported, built-in — the compiler's shadowing) and match sites.
 * One implementation of the rules, the compiler's; empty when the engine
 * cannot answer, and every consumer degrades quietly. */
async function declarationsOf(
  doc: TextDocument,
): Promise<engine.EngineDeclarations> {
  const cached = declCache.get(doc.uri);
  if (cached && cached.version === doc.version) return cached.decls;
  const fsPath = enginePath(doc);
  const decls = fsPath
    ? await engine.declarations(
        currentCompiler(),
        fsPath,
        doc.getText(),
        logEngine,
      )
    : { variants: [], matches: [] };
  declCache.set(doc.uri, { version: doc.version, decls });
  return decls;
}

/** Render a case the way completion shows it: `Variant.Tag(field: ty, ...)`. */
function caseSignature(
  e: engine.EngineVariantDecl,
  c: engine.EngineCaseDecl,
): string {
  if (c.unit && c.fields.length === 0) return `${e.name}.${c.tag}`;
  const fields = c.fields
    .map((f) => `${f.name}${f.optional ? "?" : ""}: ${f.ty}`)
    .join(", ");
  return `${e.name}.${c.tag}(${fields})`;
}

// ------------------------------------------------------------- the engine

/** The compiler path last resolved from settings — the engine session is
 * keyed by it, and the synchronous paths (document sync) cannot await
 * settings. */
let servedCompiler: string | null = null;

function currentCompiler(): string {
  return servedCompiler ?? ttc.findCompiler("", workspaceRoots);
}

/**
 * Resolves the compiler from configuration and remembers it. One engine
 * session serves the window, so the setting is read at the first workspace
 * folder's scope; a folder that configures its own path is served the
 * moment one of its files is validated ([validate]).
 *
 * The engine session is keyed by the compiler, so a change of path starts a
 * new session by itself; every open buffer is re-sent to it
 * (`setOnSessionStart`).
 */
async function refreshCompiler(): Promise<void> {
  const scope = workspaceRoots[0]
    ? URI.file(workspaceRoots[0]).toString()
    : undefined;
  const settings = await getSettings(scope);
  servedCompiler = ttc.findCompiler(settings.compilerPath, workspaceRoots);
}

const logEngine = (message: string) => connection.console.warn(message);

/** A fresh engine session starts with the disk's view of the world; hand it
 * every buffer the editor holds open. Runs on first spawn and on respawn
 * after a crash — recovery the old in-process pipeline never had. */
engine.setOnSessionStart(() => {
  const compiler = currentCompiler();
  for (const doc of documents.all()) {
    const uri = URI.parse(doc.uri);
    if (uri.scheme === "file") {
      engine.openDocument(compiler, uri.fsPath, doc.getText());
    }
  }
});

/** The open document's path when the engine can answer about it. */
function enginePath(doc: TextDocument): string | null {
  const uri = URI.parse(doc.uri);
  return uri.scheme === "file" ? uri.fsPath : null;
}

// ------------------------------------------------------------- diagnostics

const pendingValidation = new Map<string, NodeJS.Timeout>();
const VALIDATION_DELAY_MS = 300;
const validationGeneration = new Map<string, number>();

/* ------------------------------------------------------------------------
 * The typed tt layer.
 *
 * `ttc --check` answers from the text; what needs a *type* to decide — a
 * mutation through a `val` binding, exhaustiveness over the type a scrutinee
 * actually has — is the engine's typed pass. The engine keeps the compiler
 * session alive and answers incrementally (TASK-076). All enabled layers are
 * collected for one validation generation and published together: an LSP
 * diagnostic notification replaces the file's complete list, so publishing a
 * partial generation would temporarily erase every slower-layer diagnostic.
 *
 * A result computed for an older generation is not shown. The previous list
 * stays in the editor while the new generation is pending, and VS Code keeps
 * its ranges attached to the edited text until the replacement arrives.
 * --------------------------------------------------------------------- */
interface TypedDiagnostics {
  diagnostics: Diagnostic[];
  replacesTypes: boolean;
}
let warnedTypedCheckUnavailable = false;
let warnedTypedCompilerFailure = false;

/**
 * Adds the typed diagnostics that say something new.
 *
 * The two passes overlap: variant exhaustiveness is decided from the text by
 * `--check` and from the type by the typed pass, and both report it at the
 * same place. One squiggle per position: the authoritative compiler result
 * replaces the matching provisional checker result before the generation is
 * published.
 */
function mergeTyped(
  into: Diagnostic[],
  typed: Diagnostic[],
  replacesTypes = false,
): void {
  if (replacesTypes) {
    // Language-service diagnostics are the fast provisional layer. Once the
    // compiler answers with its structured checker diagnostics, replace that
    // layer as a whole so consequences suppressed by the compiler cannot
    // remain visible in the editor.
    for (let i = into.length - 1; i >= 0; i--) {
      if (into[i].source === "ts") into.splice(i, 1);
    }
  }
  const positionKey = (d: Diagnostic) =>
    `${d.range.start.line}:${d.range.start.character}`;
  const codeKey = (d: Diagnostic) => String(d.code ?? "").replace(/^ts/, "");
  for (const d of typed) {
    const sameDiagnostic = into.findIndex(
      (base) =>
        positionKey(base) === positionKey(d) &&
        codeKey(base) !== "" &&
        codeKey(base) === codeKey(d),
    );
    if (sameDiagnostic >= 0) {
      // The service answer is provisional. The compiler pass carries the
      // structured TT rendering and replaces the same checker diagnostic.
      into[sameDiagnostic] = d;
      continue;
    }
    if (into.some((base) => positionKey(base) === positionKey(d))) continue;
    into.push(d);
  }
}

async function typedDiagnosticsFor(
  doc: TextDocument,
  compiler: string,
  includeTypes: boolean,
  includeTypedTt: boolean,
): Promise<TypedDiagnostics | null> {
  const uri = URI.parse(doc.uri);
  if (uri.scheme !== "file") return null;
  const result = await ttc.runTypedCheck(
    compiler,
    doc.getText(),
    uri.fsPath,
    includeTypes,
  );

  if (result.kind === "unavailable") {
    if (result.cause === "internal") {
      if (!warnedTypedCompilerFailure) {
        warnedTypedCompilerFailure = true;
        connection.console.error(`tt: typed compiler failure: ${result.detail}`);
        void connection.window.showErrorMessage(
          "tt: typed checks failed inside the compiler. " +
            "See the tt output channel for details and report this at " +
            "https://github.com/load28/tt/issues.",
        );
      }
      return null;
    }
    // A project with no TypeScript toolchain is a normal state: the
    // text-level diagnostics keep working and only typed facts are absent.
    if (!warnedTypedCheckUnavailable) {
      warnedTypedCheckUnavailable = true;
      connection.console.info(
        `tt: typed checks unavailable (${result.detail}). ` +
          "`val` mutations and typed exhaustiveness are unavailable; " +
          "set tt.typedChecks to false to stop trying.",
      );
    }
    return null;
  }

  return {
    replacesTypes: includeTypes,
    diagnostics: result.diagnostics
      .filter((d) => includeTypedTt || String(d.code ?? "").startsWith("ts"))
      .map((d) => toDiagnostic(doc, d)),
  };
}

function scheduleValidation(doc: TextDocument): void {
  const existing = pendingValidation.get(doc.uri);
  if (existing !== undefined) clearTimeout(existing);
  const generation = (validationGeneration.get(doc.uri) ?? 0) + 1;
  validationGeneration.set(doc.uri, generation);
  pendingValidation.set(
    doc.uri,
    setTimeout(() => {
      pendingValidation.delete(doc.uri);
      void validate(doc, generation);
    }, VALIDATION_DELAY_MS),
  );
}

function isCurrentValidation(doc: TextDocument, generation: number): boolean {
  const current = documents.get(doc.uri);
  return (
    current?.version === doc.version &&
    validationGeneration.get(doc.uri) === generation
  );
}

async function validate(doc: TextDocument, generation: number): Promise<void> {
  const settings = await getSettings(doc.uri);
  const compiler = ttc.findCompiler(settings.compilerPath, workspaceRoots);
  const uri = URI.parse(doc.uri);
  const docName = uri.scheme === "file" ? uri.fsPath : uri.path;
  servedCompiler = compiler;

  const result = await ttc.runCheck(
    compiler,
    doc.getText(),
    docName,
    settings.verify,
  );

  // The buffer may have changed while ttc ran; drop stale results.
  if (!isCurrentValidation(doc, generation)) return;
  const current = documents.get(doc.uri)!;

  if (result.kind === "not-found") {
    if (!warnedCompilerMissing) {
      warnedCompilerMissing = true;
      connection.console.warn(
        `tt: compiler not found (${result.compiler}). ` +
          "Set tt.compilerPath, build target/{debug,release}/ttc, or put ttc on PATH — " +
          "diagnostics are disabled until then.",
      );
      void connection.window.showWarningMessage(
        "tt: ttc compiler not found — diagnostics are disabled. " +
          "Set `tt.compilerPath` or install ttc (cargo install --path .).",
      );
    }
    void connection.sendDiagnostics({
      uri: doc.uri,
      version: doc.version,
      diagnostics: [],
    });
    return;
  }
  if (result.kind === "failed") {
    connection.console.error(`tt: ${result.detail}`);
    void connection.sendDiagnostics({
      uri: doc.uri,
      version: doc.version,
      diagnostics: [],
    });
    return;
  }

  const diagnostics = result.diagnostics.map((d) => toDiagnostic(current, d));
  const typed =
    settings.typedChecks || settings.typeDiagnostics
      ? typedDiagnosticsFor(
          doc,
          compiler,
          settings.typeDiagnostics,
          settings.typedChecks,
        )
      : Promise.resolve(null);
  const serviceTypes = settings.typeDiagnostics
    ? typeDiagnostics(doc, compiler)
    : Promise.resolve([]);
  // Hints are not diagnostics of the compile: ttc never fails on one, and
  // they only reach the user here (`engine::hints`). Run every slower layer
  // together, then publish one complete generation.
  const [typedResult, typeResults, hints] = await Promise.all([
    typed,
    serviceTypes,
    ttHints(doc, compiler),
  ]);
  if (!isCurrentValidation(doc, generation)) return;

  diagnostics.push(...typeResults, ...hints);
  if (typedResult !== null) {
    mergeTyped(
      diagnostics,
      typedResult.diagnostics,
      typedResult.replacesTypes,
    );
  }
  void connection.sendDiagnostics({
    uri: doc.uri,
    version: doc.version,
    diagnostics,
  });
}

/**
 * TypeScript's type errors for a buffer, already on the `.tt` source.
 *
 * The engine computes them over the buffer's projection and drops anything
 * landing in compiler glue — ttc's emissions are the compiler's
 * responsibility, never something to report at the user (CLAUDE.md, error
 * layers). This side only converts and version-gates.
 */
async function typeDiagnostics(
  doc: TextDocument,
  compiler: string,
): Promise<Diagnostic[]> {
  const fsPath = enginePath(doc);
  if (fsPath === null) return [];
  const items = await engine.tsDiagnostics(compiler, fsPath, logEngine);
  return items.map((d) => ({
    severity: d.warning
      ? DiagnosticSeverity.Warning
      : DiagnosticSeverity.Error,
    range: d.range,
    message: d.message,
    code: d.code,
    source: "ts",
    // The compiler's secondary labeled spans, as the LSP's own related
    // information — the editor renders each as a clickable "here" link
    // under the diagnostic.
    relatedInformation: d.related?.length
      ? d.related.map((r) => ({
          location: {
            uri: r.path ? URI.file(r.path).toString() : doc.uri,
            range: r.range,
          },
          message: r.message,
        }))
      : undefined,
  }));
}

/**
 * What tt has to say about a buffer that is not an error — an unreachable
 * arm, today. These carry `DiagnosticSeverity.Hint` and the `Unnecessary`
 * tag, which is what dims dead code: tt has no warning level and the CLI
 * never reports these, so nothing here may look like a build failure.
 */
async function ttHints(
  doc: TextDocument,
  compiler: string,
): Promise<Diagnostic[]> {
  const fsPath = enginePath(doc);
  if (fsPath === null) return [];
  const hints = await engine.ttHints(
    compiler,
    fsPath,
    doc.getText(),
    logEngine,
  );
  return hints.map((hint) => ({
    severity: DiagnosticSeverity.Hint,
    tags: [DiagnosticTag.Unnecessary],
    range: hint.range,
    message: hint.message,
    source: "tt",
  }));
}

function toDiagnostic(doc: TextDocument, d: ttc.TtcDiagnostic): Diagnostic {
  let range: Range;
  if (d.line > 0) {
    const start = { line: d.line - 1, character: Math.max(0, d.col - 1) };
    const offset = doc.offsetAt(start);
    // The compiler's own range wins: it knows the construct's extent, so
    // the squiggle covers `try parse(text)` or `match (shape)` whole.
    // Without one, the word at the position is the best guess there is.
    const reported =
      d.endLine !== undefined && d.endLine > 0 && d.endCol !== undefined
        ? { line: d.endLine - 1, character: Math.max(0, d.endCol - 1) }
        : null;
    const word = analysis.wordAt(doc.getText(), offset);
    const end =
      reported && doc.offsetAt(reported) > offset
        ? reported
        : word && word.start === offset
          ? doc.positionAt(word.end)
          : { line: start.line, character: start.character + 1 };
    range = { start, end };
  } else {
    // Positionless (output-verification) errors: flag the first line.
    range = {
      start: { line: 0, character: 0 },
      end: { line: 1, character: 0 },
    };
  }
  return {
    severity: DiagnosticSeverity.Error,
    range,
    message: d.message,
    code: d.code,
    source: "ttc",
    // The compiler's secondary labeled spans. Typed diagnostics replace
    // the language-service layer under the default settings, so the
    // related places must travel on this path too or the editor loses
    // them the moment the typed answer lands.
    relatedInformation: d.labels?.length
      ? d.labels.map((label) => ({
          location: {
            uri: label.path ? URI.file(label.path).toString() : doc.uri,
            range: {
              start: {
                line: Math.max(0, label.line - 1),
                character: Math.max(0, label.col - 1),
              },
              end: {
                line: Math.max(0, label.endLine - 1),
                character: Math.max(0, label.endCol - 1),
              },
            },
          },
          message: label.message,
        }))
      : undefined,
    // The compiler's own fixes, carried through to `onCodeAction`. LSP
    // round-trips `data` untouched, so the quick fix is the compiler's
    // answer rather than this server's reading of the message.
    data: d.suggestions?.length ? { suggestions: d.suggestions } : undefined,
  };
}

documents.onDidOpen((e) => {
  const fsPath = enginePath(e.document);
  if (fsPath !== null) {
    engine.openDocument(currentCompiler(), fsPath, e.document.getText());
  }
  scheduleValidation(e.document);
});
documents.onDidSave((e) => {
  void rebuildSidecar(e.document);
});

/**
 * Where this file's declarations belong: next to the source when
 * `tt.sidecarDir` is empty, otherwise that directory under the workspace
 * root the file lives in (TypeScript merges the two trees with `rootDirs`).
 */
function resolveSidecarDir(configured: string, filePath: string): string | undefined {
  const dir = configured.trim();
  if (dir === "") return undefined;
  if (path.isAbsolute(dir)) return dir;

  const root = workspaceRoots
    .filter((candidate) => filePath.startsWith(`${candidate}${path.sep}`))
    .sort((a, b) => b.length - a.length)[0];
  return root === undefined ? undefined : path.join(root, dir);
}

/**
 * Keeps a saved `.tt` file's editor sidecar (`x.tt.d.ts` + map) current, so
 * `.ts` files importing it type-check and jump into the original on "go to
 * definition" without a build step.
 */
async function rebuildSidecar(doc: TextDocument): Promise<void> {
  const uri = URI.parse(doc.uri);
  if (uri.scheme !== "file") return;

  const settings = await getSettings(doc.uri);
  if (settings.sidecar === "off") return;

  const compiler = ttc.findCompiler(settings.compilerPath, workspaceRoots);
  const result = await sidecar.refreshSidecar(
    compiler,
    uri.fsPath,
    settings.sidecar,
    resolveSidecarDir(settings.sidecarDir, uri.fsPath),
  );
  if (result.kind === "failed") {
    connection.console.warn(`tt: sidecar refresh failed — ${result.detail}`);
  }
}
documents.onDidChangeContent((e) => {
  const fsPath = enginePath(e.document);
  if (fsPath !== null) {
    engine.updateDocument(currentCompiler(), fsPath, e.document.getText());
  }
  // Editing one file can change what its siblings import — drop their
  // cached declaration surfaces (the edited doc's own entry refreshes by
  // version).
  for (const uri of declCache.keys()) {
    if (uri !== e.document.uri) declCache.delete(uri);
  }
  scheduleValidation(e.document);
});
documents.onDidClose((e) => {
  const fsPath = enginePath(e.document);
  if (fsPath !== null) {
    engine.closeDocument(currentCompiler(), fsPath);
  }
  analysisCache.delete(e.document.uri);
  declCache.delete(e.document.uri);
  validationGeneration.delete(e.document.uri);
  const pending = pendingValidation.get(e.document.uri);
  if (pending !== undefined) {
    clearTimeout(pending);
    pendingValidation.delete(e.document.uri);
  }
  void connection.sendDiagnostics({
    uri: e.document.uri,
    version: e.document.version,
    diagnostics: [],
  });
});

// -------------------------------------------------------------- completion

const KEYWORD_SNIPPETS: CompletionItem[] = [
  {
    label: "variant",
    kind: CompletionItemKind.Snippet,
    detail: "tt variant declaration",
    documentation: {
      kind: MarkupKind.Markdown,
      value:
        "tt 태그드 유니언 선언. 유닛 케이스는 괄호 없이 값으로 선언할 수 있습니다.",
    },
    insertTextFormat: InsertTextFormat.Snippet,
    insertText: "variant ${1:Name} {\n\t${2:Case}(${3:field}: ${4:number}),\n\t${5:Unit},\n}",
  },
  {
    label: "match",
    kind: CompletionItemKind.Snippet,
    detail: "tt match expression",
    documentation: {
      kind: MarkupKind.Markdown,
      value:
        "`kind` 태그로 분기하는 match 표현식. `_` 없는 match는 같은 파일·import한 tt variant에 대해 소진성이 검사됩니다.",
    },
    insertTextFormat: InsertTextFormat.Snippet,
    insertText: "match (${1:value}) {\n\t$0\n}",
  },
  {
    label: "try",
    kind: CompletionItemKind.Snippet,
    detail: "tt try statement (Result propagation)",
    documentation: {
      kind: MarkupKind.Markdown,
      value:
        "Rust의 `?`에 해당. `Err`면 둘러싼 함수에서 즉시 return합니다. 세미콜론 필수.",
    },
    insertTextFormat: InsertTextFormat.Snippet,
    insertText: "try ${1:expression};",
  },
  {
    label: "flow",
    kind: CompletionItemKind.Snippet,
    detail: "tt flow composition (point-free pipeline)",
    documentation: {
      kind: MarkupKind.Markdown,
      value:
        "값 대신 함수를 합성해 새 함수를 만듭니다. 첫 스텝이 입력 타입을 정하며, 메서드 스텝이 될 수 없습니다.",
    },
    insertTextFormat: InsertTextFormat.Snippet,
    insertText: "flow |> ${1:first} |> ${0:next}",
  },
  {
    label: "result",
    kind: CompletionItemKind.Snippet,
    detail: "tt result computation block",
    documentation: {
      kind: MarkupKind.Markdown,
      value:
        "`Result` 연산을 평탄하게 잇습니다. `const x <- 식;`은 `Ok` 값을 묶고 `Err`를 블록 밖으로 전파하며, 마지막 값 식(세미콜론 없이)이 `Ok`로 감싸집니다.",
    },
    insertTextFormat: InsertTextFormat.Snippet,
    insertText: "result {\n\tconst ${1:value} <- ${2:expression};\n\t$0\n}",
  },
  {
    label: "let-else",
    kind: CompletionItemKind.Snippet,
    detail: "tt let-else statement",
    documentation: {
      kind: MarkupKind.Markdown,
      value:
        "패턴이 일치하면 필드를 바인딩하고, 아니면 발산하는 else 블록을 실행합니다. 괄호와 세미콜론 필수.",
    },
    insertTextFormat: InsertTextFormat.Snippet,
    insertText:
      "const ${1:Some}(${2:value}) = ${3:expression} else {\n\t${4:return;}\n};",
  },
];

/** TypeScript element-kind strings → LSP completion kinds. */
const TS_COMPLETION_KINDS: Record<string, CompletionItemKind> = {
  var: CompletionItemKind.Variable,
  let: CompletionItemKind.Variable,
  const: CompletionItemKind.Variable,
  "local var": CompletionItemKind.Variable,
  parameter: CompletionItemKind.Variable,
  alias: CompletionItemKind.Reference,
  function: CompletionItemKind.Function,
  "local function": CompletionItemKind.Function,
  method: CompletionItemKind.Method,
  property: CompletionItemKind.Property,
  getter: CompletionItemKind.Property,
  setter: CompletionItemKind.Property,
  class: CompletionItemKind.Class,
  interface: CompletionItemKind.Interface,
  type: CompletionItemKind.TypeParameter,
  enum: CompletionItemKind.Enum,
  "enum member": CompletionItemKind.EnumMember,
  module: CompletionItemKind.Module,
  keyword: CompletionItemKind.Keyword,
  string: CompletionItemKind.Constant,
};

/** What a TS-delegated completion item carries so its signature and
 * documentation can be fetched when the editor asks for that one entry
 * (`completionItem/resolve`). Positions are in source coordinates and the
 * engine re-maps at resolve time — the buffer may have moved on since. */
interface TsCompletionData {
  uri: string;
  offset: number;
  name: string;
  /** The engine's probe the entry was listed from, when it came from one —
   * the detail must be fetched against that same text. */
  probe?: number;
}

/**
 * True when `offset` sits right after a member-access dot. `masked` is the
 * masked source (analysis.ts), so a `.` inside a string, comment or regex
 * is not one.
 */
function atMemberAccess(masked: string, offset: number): boolean {
  return offset > 0 && masked[offset - 1] === ".";
}

/**
 * TypeScript completions from the engine, sorted after the tt-specific
 * items (`2` prefix; tt items use `0`/`1`). The engine applies the whole
 * member/probe policy: at a member access only a member answer comes back,
 * probed from the mended source when the buffer's own text cannot answer.
 */
async function tsCompletions(
  doc: TextDocument,
  offset: number,
  atMember: boolean,
): Promise<CompletionItem[]> {
  const fsPath = enginePath(doc);
  if (fsPath === null) return [];
  const list = await engine.completion(
    currentCompiler(),
    fsPath,
    doc.positionAt(offset),
    atMember,
    logEngine,
  );
  if (!list) return [];
  return list.items.map((entry) => ({
    label: entry.label,
    kind: TS_COMPLETION_KINDS[entry.kind] ?? CompletionItemKind.Text,
    sortText: `2${entry.sortText}`,
    data: {
      uri: doc.uri,
      offset,
      name: entry.label,
      probe: list.probe ?? undefined,
    } satisfies TsCompletionData,
  }));
}

connection.onCompletion(async (params): Promise<CompletionItem[]> => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return [];
  const { masked } = analyze(doc);
  const offset = doc.offsetAt(params.position);
  const visible = (await declarationsOf(doc)).variants;

  // `Variant.` member access → the variant's case constructors, then everything
  // else TypeScript offers on that same object. Both halves are needed:
  // the constructors are tt's (case signature, field snippet, tags an
  // unimported built-in still has), while the rest of a standard-library
  // namespace — `Result.map`, `Option.unwrapOrElse`, the `*P` pipeline
  // variants — is ordinary TypeScript the service already types. Returning
  // only the constructors hid every combinator behind `Result.`/`Option.`
  // (TASK-062). Any other member access (`obj.`) is TypeScript's alone.
  const base = analysis.memberAccessAt(masked, offset);
  if (base !== null) {
    const e = visible.find((x) => x.name === base);
    const members = await tsCompletions(doc, offset, true);
    if (!e) return members;
    const items = e.cases.map((c) => constructorItem(e, c));
    const tags = new Set(items.map((i) => i.label));
    return items.concat(members.filter((i) => !tags.has(i.label)));
  }

  // A `.` with no identifier in front of it is a member access all the same
  // (`x |> .`, `f().`, `(a + b).`): members belong there, and nothing else —
  // no variant names, no keyword snippets.
  if (atMemberAccess(masked, offset)) {
    return tsCompletions(doc, offset, true);
  }

  // A pattern position — an arm, an `if let`, a payload field list, a
  // nested pattern — is tt's alone: case tags and field names exist
  // nowhere in the emitted TypeScript, so the service has nothing to
  // complete there. The engine answers from the compiler's own
  // declaration table, under the same shadowing the compiler resolves
  // with, and knows the positions this server never did (`if let`,
  // let-else payloads, nested patterns).
  const fsPath = enginePath(doc);
  const ttItemsHere = fsPath
    ? await engine.ttCompletions(
        currentCompiler(),
        fsPath,
        doc.getText(),
        params.position,
        logEngine,
      )
    : [];
  if (ttItemsHere.length > 0) {
    return ttItemsHere.map((item) => ({
      label: item.label,
      kind:
        item.kind === "case"
          ? CompletionItemKind.EnumMember
          : item.kind === "field"
            ? CompletionItemKind.Field
            : CompletionItemKind.Keyword,
      detail: item.detail,
      // An arm already written stays in the list — a guard may repeat a
      // tag — but sorts after the ones still missing.
      sortText: `${item.covered ? 1 : 0}${item.label}`,
    }));
  }

  // General position → variant names + tt keyword snippets, then everything
  // TypeScript would offer in a .ts file (sorted after the tt items).
  const items: CompletionItem[] = visible.map((e) => ({
    label: e.name,
    kind: CompletionItemKind.Enum,
    detail:
      e.origin === "builtin"
        ? `내장 variant ${e.name}${e.generics}`
        : e.origin === "imported"
          ? `variant ${e.name}${e.generics}${e.specifier ? ` — ${e.specifier}` : ""}`
          : `variant ${e.name}${e.generics}`,
    sortText: `0${e.name}`,
  }));
  const ttItems = items.concat(KEYWORD_SNIPPETS);
  const seen = new Set(ttItems.map((i) => i.label));
  return ttItems.concat(
    (await tsCompletions(doc, offset, false)).filter((i) => !seen.has(i.label)),
  );
});

/**
 * The signature and documentation of a delegated completion entry, fetched
 * when the editor asks about the one entry the user is looking at. This is
 * what puts a type on `Result.map` in the completion list; tt's own items
 * carry their signature in `detail` from the start and pass through here
 * untouched.
 */
connection.onCompletionResolve(
  async (item: CompletionItem): Promise<CompletionItem> => {
    const data = item.data as TsCompletionData | undefined;
    if (!data || typeof data.uri !== "string") return item;
    const doc = documents.get(data.uri);
    if (!doc) return item;
    const fsPath = enginePath(doc);
    if (fsPath === null) return item;
    const detail = await engine.completionResolve(
      currentCompiler(),
      fsPath,
      doc.positionAt(data.offset),
      data.name,
      data.probe,
      logEngine,
    );
    if (!detail) return item;
    if (detail.signature !== "") item.detail = detail.signature;
    if (detail.documentation !== "") {
      item.documentation = {
        kind: MarkupKind.Markdown,
        value: detail.documentation,
      };
    }
    return item;
  },
);

// ---------------------------------------------------------- signature help

/**
 * Parameter hints while a call is being written, delegated wholesale to
 * the engine — so the arguments of a standard library combinator
 * (`Result.andThen(r, f)`) are typed as they are typed, including inside a
 * `match` arm or a `|>` pipeline, where the raw text is not TypeScript at
 * all. tt syntax that has no call at the cursor simply yields nothing.
 */
connection.onSignatureHelp(async (params): Promise<SignatureHelp | null> => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return null;
  const fsPath = enginePath(doc);
  if (fsPath === null) return null;
  const help = await engine.signatureHelp(
    currentCompiler(),
    fsPath,
    params.position,
    logEngine,
  );
  if (!help || help.signatures.length === 0) return null;
  return {
    signatures: help.signatures.map(
      (sig): SignatureInformation => ({
        label: sig.label,
        documentation: sig.documentation
          ? { kind: MarkupKind.Markdown, value: sig.documentation }
          : undefined,
        parameters: sig.parameters.map(
          (p): ParameterInformation => ({
            label: p.label,
            documentation: p.documentation
              ? { kind: MarkupKind.Markdown, value: p.documentation }
              : undefined,
          }),
        ),
      }),
    ),
    activeSignature: help.activeSignature,
    activeParameter: help.activeParameter,
  };
});

function constructorItem(
  e: engine.EngineVariantDecl,
  c: engine.EngineCaseDecl,
): CompletionItem {
  const unit = c.unit && c.fields.length === 0;
  const item: CompletionItem = {
    label: c.tag,
    kind: unit ? CompletionItemKind.EnumMember : CompletionItemKind.Constructor,
    detail: caseSignature(e, c),
    sortText: `0${c.tag}`,
  };
  if (!unit) {
    item.insertTextFormat = InsertTextFormat.Snippet;
    item.insertText =
      c.fields.length > 0
        ? `${c.tag}(${c.fields.map((f, i) => `\${${i + 1}:${f.name}}`).join(", ")})`
        : `${c.tag}()`;
  }
  return item;
}

// ------------------------------------------------------------------- hover

connection.onHover(async (params) => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return null;
  const { text } = analyze(doc);
  const offset = doc.offsetAt(params.position);

  const w = analysis.wordAt(text, offset);
  if (w && w.word === "match") {
    const m = (await declarationsOf(doc)).matches.find(
      (site) => site.keyword === w.start,
    );
    if (m) {
      return {
        contents: {
          kind: MarkupKind.Markdown,
          value:
            "```tt\nmatch (값) { 패턴 => 본문, ... }\n```\n" +
            "tt match 표현식 — 값의 `kind` 필드로 분기합니다. " +
            "`_` 없는 match는 같은 파일·import한 tt variant(내장 `Option`/`Result` 포함)에 대해 소진성이 검사됩니다.",
        },
        range: { start: doc.positionAt(w.start), end: doc.positionAt(w.end) },
      };
    }
  }

  // A tt name — a variant, a case tag, a payload field. None of the three
  // survives lowering in a form the TypeScript service can be pointed at,
  // so the engine answers from the compiler's own declaration table. It
  // answers only where the service cannot be asked; everywhere else
  // (`Shape.Circle(1)`, `const s: Shape`) the service knows more.
  const fsPath = enginePath(doc);
  const sym = fsPath
    ? await engine.ttSymbol(
        currentCompiler(),
        fsPath,
        doc.getText(),
        params.position,
        logEngine,
      )
    : null;
  if (!sym) return tsHover(doc, params.position);
  return {
    contents: {
      kind: MarkupKind.Markdown,
      value: `\`\`\`tt\n${sym.signature}\n\`\`\`\n${sym.detail}`,
    },
    range: sym.range,
  };
});

/** Hover for everything the tt layer does not own, from the engine. */
async function tsHover(
  doc: TextDocument,
  position: { line: number; character: number },
) {
  const fsPath = enginePath(doc);
  if (fsPath === null) return null;
  const info = await engine.hover(currentCompiler(), fsPath, position, logEngine);
  if (!info || info.signature === "") return null;
  const value =
    "```ts\n" +
    info.signature +
    "\n```" +
    (info.documentation ? `\n${info.documentation}` : "");
  return {
    contents: { kind: MarkupKind.Markdown, value },
    range: info.range,
  };
}

// -------------------------------------------------------------- definition

connection.onDefinition(async (params) => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return null;

  // A tt name first: a variant, a case tag, a payload field. The engine
  // knows where each is declared — in this file or in the `.tt` the import
  // names — because the emitted TypeScript carries none of them.
  const ttPath = enginePath(doc);
  if (ttPath !== null) {
    const sym = await engine.ttSymbol(
      currentCompiler(),
      ttPath,
      doc.getText(),
      params.position,
      logEngine,
    );
    if (sym?.definition) {
      return Location.create(
        URI.file(sym.definition.path).toString(),
        sym.definition.range,
      );
    }
    // A built-in case has no declaration to open; nothing else does either
    // once the engine has claimed the position.
    if (sym) return null;
  }

  // Everything else — ordinary TypeScript symbols, and built-in variant
  // names the user may have imported from the std module — is the
  // engine's answer, already in `.tt` coordinates.
  const fsPath = enginePath(doc);
  if (fsPath === null) return null;
  const locations = await engine.definition(
    currentCompiler(),
    fsPath,
    params.position,
    logEngine,
  );
  if (locations.length === 0) return null;
  return locations.map((l) =>
    Location.create(URI.file(l.path).toString(), l.range),
  );
});

// -------------------------------------------------- references and rename

connection.onReferences(async (params): Promise<Location[] | null> => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return null;
  const fsPath = enginePath(doc);
  if (fsPath === null) return null;
  // Delegated wholesale to the engine: TypeScript resolves the passthrough
  // region exactly, and tt-specific spans degrade to an empty result.
  const references = (
    await engine.references(currentCompiler(), fsPath, params.position, logEngine)
  ).filter((r) => params.context.includeDeclaration || !r.isDefinition);
  if (references.length === 0) return null;
  return references.map((r) =>
    Location.create(URI.file(r.path).toString(), r.range),
  );
});

connection.onRenameRequest(async (params) => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return null;
  const fsPath = enginePath(doc);
  if (fsPath === null) return null;

  // tt names (variants, case tags, payload fields) are compiled into emitted
  // `kind` strings and destructuring keys — renaming one needs tt-aware
  // rewriting across both worlds, so refuse rather than let TypeScript do
  // half the job. The engine decides what is a tt name; this server does
  // not keep a second opinion.
  const sym = await engine.ttSymbol(
    currentCompiler(),
    fsPath,
    doc.getText(),
    params.position,
    logEngine,
  );
  if (sym) return null;

  // The engine owns the safety rule: an edit that cannot be mapped back to
  // source would corrupt the rename, so such a rename comes back null —
  // whole or not at all.
  const edits = await engine.rename(
    currentCompiler(),
    fsPath,
    params.position,
    logEngine,
  );
  if (!edits || edits.length === 0) return null;
  const changes: Record<string, TextEdit[]> = {};
  for (const edit of edits) {
    // What the engine wants written, which is not always the bare name: a
    // destructuring shorthand — what a tt pattern binding `Some(value)`
    // compiles to — expands to `value: <new>`, and dropping the expansion
    // would rebind a *different* field under the new name.
    const newText =
      edit.newText === null
        ? params.newName
        : edit.newText.split(engine.RENAME_PLACEHOLDER).join(params.newName);
    const target = URI.file(edit.path).toString();
    (changes[target] ??= []).push(TextEdit.replace(edit.range, newText));
  }
  return { changes };
});

// ----------------------------------------------------------------- symbols

connection.onDocumentSymbol(async (params): Promise<DocumentSymbol[]> => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return [];
  const decls = await declarationsOf(doc);
  const out: DocumentSymbol[] = [];
  for (const e of decls.variants) {
    // Only this file's declarations belong in its outline.
    if (e.origin !== "local" || !e.span || !e.nameSpan) continue;
    const range = {
      start: doc.positionAt(e.span.start),
      end: doc.positionAt(e.span.end),
    };
    out.push({
      name: `${e.name}${e.generics}`,
      kind: SymbolKind.Enum,
      range,
      selectionRange: {
        start: doc.positionAt(e.nameSpan.start),
        end: doc.positionAt(e.nameSpan.end),
      },
      children: e.cases.flatMap((c) => {
        if (!c.span) return [];
        const tagRange = {
          start: doc.positionAt(c.span.start),
          end: doc.positionAt(c.span.end),
        };
        return [
          {
            name: c.tag,
            detail:
              c.fields.length > 0
                ? `(${c.fields.map((f) => f.name).join(", ")})`
                : undefined,
            kind: SymbolKind.EnumMember,
            range: tagRange,
            selectionRange: tagRange,
          },
        ];
      }),
    });
  }
  return out;
});

// ------------------------------------------------------------ code actions

/**
 * The compiler's own fixes, off the diagnostic's `data`.
 *
 * A suggestion that names a replacement is a quick fix outright — the span
 * and the text both come from the compiler, so nothing here has to
 * recognise a message by its shape.
 */
function suggestedFixes(doc: TextDocument, diag: Diagnostic): CodeAction[] {
  const data = diag.data as { suggestions?: ttc.TtcSuggestion[] } | undefined;
  const actions: CodeAction[] = [];
  for (const suggestion of data?.suggestions ?? []) {
    const edit = suggestion.edit;
    if (!edit) continue;
    const range: Range = {
      start: { line: edit.line - 1, character: Math.max(0, edit.col - 1) },
      end: {
        line: edit.endLine - 1,
        character: Math.max(0, edit.endCol - 1),
      },
    };
    actions.push({
      // The compiler's own sentence names the fix. A title built from the
      // replacement text was readable while every fix was one identifier;
      // an inserted arm block is not a title (TASK-216).
      title: suggestion.message,
      kind: CodeActionKind.QuickFix,
      diagnostics: [diag],
      isPreferred: actions.length === 0,
      edit: {
        changes: {
          [doc.uri]: [{ range, newText: edit.replacement }],
        },
      },
    });
  }
  return actions;
}

connection.onCodeAction(async (params): Promise<CodeAction[]> => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return [];
  const actions: CodeAction[] = [];
  for (const diag of params.context.diagnostics) {
    if (diag.source !== "ttc") continue;
    actions.push(...suggestedFixes(doc, diag));
  }
  return actions;
});

// -------------------------------------------------------- semantic tokens

/** The legend, fixed at initialize: the LSP standard token types the engine
 * reports (engine.ts `EngineSemanticToken.kind`), in the order the encoded
 * data indexes them. */
const SEMANTIC_TOKEN_TYPES = [
  "keyword",
  "enum",
  "enumMember",
  "variable",
  "property",
  "function",
  "operator",
];

connection.languages.semanticTokens.on(async (params) => {
  const doc = documents.get(params.textDocument.uri);
  if (!doc) return { data: [] };
  // Text-based and parse-only on the engine side: it answers for unsaved
  // and untitled buffers alike, with or without a TypeScript toolchain.
  const tokens = await engine.semanticTokens(
    currentCompiler(),
    doc.getText(),
    URI.parse(doc.uri).scheme === "file"
      ? URI.parse(doc.uri).fsPath
      : `buffer.${doc.languageId === "ttx" ? "ttx" : "tt"}`,
    logEngine,
  );
  // Engine unavailable: no answer beats a wrong empty one — the grammar's
  // colors stand alone, exactly as they do for every other engine feature.
  if (!tokens) return { data: [] };

  const builder = new SemanticTokensBuilder();
  const ordered = tokens
    .map((token) => ({
      line: token.range.start.line,
      character: token.range.start.character,
      length: token.range.end.character - token.range.start.character,
      type: SEMANTIC_TOKEN_TYPES.indexOf(token.kind),
    }))
    .filter((token) => token.type >= 0 && token.length > 0)
    .sort((a, b) => a.line - b.line || a.character - b.character);
  for (const token of ordered) {
    builder.push(token.line, token.character, token.length, token.type, 0);
  }
  return builder.build();
});

// ------------------------------------------------------------------- start

documents.listen(connection);
connection.listen();
