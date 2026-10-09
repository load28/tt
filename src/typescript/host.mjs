/* --------------------------------------------------------------------------
 * host.mjs — the TypeScript 7 native backend's host process.
 *
 * ttc embeds this file (`include_str!`) and runs it with `node`. It is the
 * only place that knows the TypeScript API: it opens ONE real TypeScript
 * project over a layered file system where every `.tt` file appears as the
 * ordinary TypeScript it lowers to, and answers tt's semantic questions
 * against that project's checker — or, for a root module that project does
 * not contain, against the module's default project.
 *
 * The API comes from the TypeScript the project installed (see
 * `toolchain.rs`): the JS client and the native executable speak an
 * unversioned MessagePack protocol and must come from the same build, so
 * the client is named and it runs the executable shipped beside it.
 *
 * The host is a **session**: one line of JSON in, one line of JSON out, for
 * as long as stdin stays open. The compiler is started once and the project
 * is opened once; a later request only says what changed. That is what makes
 * a watch or an editor viable — reopening a real project per keystroke is
 * not.
 *
 *   open   { apiModule, cwd, tsconfig (nullable), outputs: [dir] }
 *       →  { ok: true }
 *
 *   ask    { modules: [{ path, text }],   // lowered .tt → x.tt.ts / x.ttx.tsx
 *            roots: [path],               // requested and open modules
 *            literalChecks: [{ module, start, covered: [...] }],
 *            tagChecks: [{ module, start, covered: [...] }],
 *            symbolChecks: [{ module, start, binding }],
 *            resultShapeChecks: [{ module, start, end }],
 *            emitDeclarations: boolean }
 *       →  { diagnostics: [{ file, start, end, code, message, mismatch? }],
 *            literalMissing: [{ index, missing }],
 *            tagMissing: [{ index, missing }],
 *            tagMembers: [{ index, tags }],
 *            symbols: [{ index, id, name, builtin }],
 *            resultShapes: [{ index }],
 *            declarations: [{ path, text }],
 *            contextualRoundTrips }      // checker requests the
 *                                        // contextual pass sent
 *
 * An `ask` may also answer `{ error: "..." }`, which fails that request
 * without ending the session. EOF on stdin ends it.
 *
 * While it answers a request, the host may ask ttc which listed files are
 * ttc's own outputs, which no directory listing admits to the program:
 *
 *   ←  { ownedOutputs: [path] }
 *   →  { owned: [path] }
 *
 * `start`/`end` are UTF-16 code-unit offsets — TypeScript's own coordinate
 * space. Mapping them back to `.tt` byte positions is ttc's job (`mapper`),
 * not this host's.
 *
 * Inside one `ask` the per-position questions are batched by module through
 * the checker's array overloads (`getTypesAtPositions`,
 * `getSymbolsAtPositions`, `getTypeOfSymbol[]`), falling back to one call
 * per question on a client without them — see `batched`. The protocol above
 * and the meaning of every answer are the same either way.
 *
 * Exit codes: 0 = ran (type errors, if any, are in `diagnostics`),
 * 2 = the TypeScript API could not be loaded, 3 = malformed job,
 * 5 = the resolved TypeScript has no declaration emit API.
 * ----------------------------------------------------------------------- */
import { createHash } from "node:crypto";
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

/**
 * How a lowered module reaches the compiler.
 *
 * A configured project opens its `.tt`/`.ttx` files through a TypeScript
 * content mapper, the extension point `tsc --runExternalCode` uses for the
 * same files. `.tt` is then a supported extension, so `"./x.tt"` resolves in
 * every `moduleResolution` mode exactly as it does for `tsc`. The mapper is
 * the identity: the engine serves the lowered text as the file's content, so
 * the virtual text and every position in it are the lowered module's own.
 * The engine keeps naming the module `x.tt.ts`; `served` and `moduleName`
 * translate at this boundary.
 *
 * A project whose configuration already names another content mapper keeps
 * the previous arrangement, lowered modules served as `x.tt.ts`, because
 * enabling external code would also run mappers the user has not trusted
 * this process to run. An inferred project has no configuration to name a
 * mapper in.
 */
const MAPPER_PACKAGE = "@tt/typed-engine-mapper";

function publishFile(file, text) {
  try {
    if (fs.readFileSync(file, "utf8") === text) return;
  } catch {}
  const staging = `${file}.${process.pid}.tmp`;
  fs.writeFileSync(staging, text);
  fs.renameSync(staging, file);
}
function jsoncTree(text) {
  let at = 0;
  const skip = () => {
    for (;;) {
      if (/\s|﻿/.test(text[at] ?? "")) at += 1;
      else if (text.startsWith("//", at)) at = text.indexOf("\n", at) < 0 ? text.length : text.indexOf("\n", at);
      else if (text.startsWith("/*", at)) {
        const close = text.indexOf("*/", at + 2);
        if (close < 0) return false;
        at = close + 2;
      } else return true;
    }
  };
  const string = () => {
    const quote = text[at];
    const start = at;
    for (at += 1; at < text.length && text[at] !== quote; at += text[at] === "\\" ? 2 : 1) {}
    if (at >= text.length) return null;
    at += 1;
    const raw = text.slice(start, at);
    let key;
    try { key = JSON.parse(quote === '"' ? raw : `"${raw.slice(1, -1).replaceAll('"', '\\"')}"`); } catch { key = raw; }
    return { kind: "string", start, end: at, key };
  };
  const startsElement = () => at < text.length && !/[,:}\]]/.test(text[at]);
  const value = () => {
    if (!skip()) return null;
    const start = at;
    const open = text[at];
    if (open === '"' || open === "'") return string();
    if (open === "{" || open === "[") {
      const close = open === "{" ? "}" : "]";
      const node = open === "{" ? { kind: "object", start, members: [] } : { kind: "array", start, elements: [] };
      at += 1;
      let previousComma = null;
      for (;;) {
        if (!skip()) return null;
        if (text[at] === close) { at += 1; node.end = at; return node; }
        if (at >= text.length) { node.end = at; return node; }
        let entry;
        if (open === "{") {
          const name = text[at] === '"' || text[at] === "'" ? string() : null;
          if (!name || !skip() || text[at] !== ":") return null;
          at += 1;
          const member = value();
          if (!member) return null;
          entry = { key: name.key, start: name.start, end: member.end, value: member, previousComma, comma: null };
          node.members.push(entry);
        } else {
          entry = value();
          if (!entry) return null;
          node.elements.push(entry);
        }
        if (!skip()) return null;
        if (text[at] === ",") {
          entry.comma = at;
          previousComma = at;
          at += 1;
        } else if (open === "{" && text[at] === ";") {
          at += 1;
        } else if (text[at] !== close && at < text.length && !startsElement()) return null;
      }
    }
    while (at < text.length && !/[\s,:{}[\]"'/]/.test(text[at])) at += 1;
    return at > start ? { kind: "literal", start, end: at } : null;
  };
  if (skip() && at === text.length) return { kind: "empty", start: at, end: at, members: [] };
  const root = value();
  return root && skip() && at === text.length ? root : null;
}

function editedText(text, edits) {
  const ordered = [...edits].sort((a, b) => a.start - b.start);
  let served = "";
  let from = 0;
  const spans = [];
  for (const edit of ordered) {
    served += text.slice(from, edit.start);
    spans.push({ servedStart: served.length, servedEnd: served.length + edit.text.length, start: edit.start, end: edit.end });
    served += edit.text;
    from = edit.end;
  }
  return { text: served + text.slice(from), spans };
}

function originalSpan(spans, start, end) {
  const inserted = spans.find((span) =>
    span.start === span.end && span.servedStart <= start && end <= span.servedEnd && span.servedStart < span.servedEnd);
  if (inserted) return null;
  const original = (offset, closing) => {
    let delta = 0;
    for (const span of spans) {
      if (offset < span.servedStart || (closing && offset === span.servedStart)) return offset + delta;
      if (offset < span.servedEnd || (closing && offset === span.servedEnd)) return closing ? span.end : span.start;
      delta = span.end - span.servedEnd;
    }
    return offset + delta;
  };
  return { start: original(start, false), end: original(end, true) };
}

const CANNOT_READ_FILE = 5083;
const RECORD = "\u001e";
const CIRCULAR_CONFIGURATION = 18000;
const CANNOT_FIND_MODULE = 2307;
const LOWERED = /\.(?:tt\.ts|ttx\.tsx)$/;
const TT_SOURCE = /\.ttx?$/;
const IDENTITY_MAPPER = `
let pending = Buffer.alloc(0);
process.stdin.on("data", (chunk) => {
  pending = Buffer.concat([pending, chunk]);
  for (;;) {
    const head = pending.indexOf("\\r\\n\\r\\n");
    if (head < 0) return;
    const length = Number(/content-length: *(\\d+)/i.exec(pending.subarray(0, head).toString("latin1"))[1]);
    if (pending.length < head + 4 + length) return;
    const message = JSON.parse(pending.subarray(head + 4, head + 4 + length).toString("utf8"));
    pending = pending.subarray(head + 4 + length);
    if (message.id === undefined) continue;
    let result = {};
    if (message.method === "initialize") result = { positionEncoding: "utf-16", diagnosticSource: "tt" };
    if (message.method === "transform") {
      const text = message.params.content;
      result = {
        text,
        extension: message.params.fileName.endsWith(".ttx") ? ".tsx" : ".ts",
        mappings: text.length > 0 ? [[0, text.length, 0, text.length, 0]] : [],
      };
    }
    const body = Buffer.from(JSON.stringify({ jsonrpc: "2.0", id: message.id, result }), "utf8");
    process.stdout.write("Content-Length: " + body.length + "\\r\\n\\r\\n");
    process.stdout.write(body);
  }
});
`;

/**
 * Writes a whole answer line to stdout **synchronously**.
 *
 * `process.stdout.write` queues anything past the pipe's buffer (64 KB on
 * Linux) for the event loop to flush — and this host then blocks the event
 * loop in `readSync` waiting for the next request, which never comes
 * because the client is still waiting for the rest of the answer. A
 * project with a few hundred diagnostics crosses 64 KB, so the flush has
 * to happen before the loop turns around: partial writes and EAGAIN are
 * retried until every byte is out.
 */
function writeLine(text) {
  const buffer = Buffer.from(RECORD + text + "\n", "utf8");
  let pos = 0;
  while (pos < buffer.length) {
    try {
      pos += fs.writeSync(1, buffer, pos, buffer.length - pos);
    } catch (e) {
      if (e.code === "EAGAIN") continue;
      throw e;
    }
  }
}

/**
 * Reads stdin one line at a time, blocking. The client waits for each
 * answer before sending the next request, so a single buffer is enough.
 */
function lineReader() {
  const buf = Buffer.alloc(65536);
  let pending = "";
  return function readLine() {
    while (true) {
      const newline = pending.indexOf("\n");
      if (newline >= 0) {
        const line = pending.slice(0, newline);
        pending = pending.slice(newline + 1);
        return line;
      }
      let n;
      try {
        n = fs.readSync(0, buf, 0, buf.length, null);
      } catch (e) {
        if (e.code === "EOF") n = 0;
        else throw e;
      }
      if (n === 0) return pending.length > 0 ? ((p) => ((pending = ""), p))(pending) : null;
      pending += buf.subarray(0, n).toString("utf8");
    }
  };
}

/**
 * The file system TypeScript sees: ttc's lowered modules layered over the
 * real disk. A `.tt` file is invisible to TypeScript; the `.ts` it lowers to
 * takes its place, including in directory listings, so a `tsconfig.json`
 * that globs a directory picks it up exactly as it would a hand-written file.
 */
function diskVersion(file) {
  try { const stat = fs.statSync(file); return `${stat.mtimeMs}:${stat.size}`; }
  catch { return null; }
}

function layeredFileSystem(files, aliases, dirs, configFiles, dependencies, listings, links, outputs, ownedOutputs) {
  // The packages this host publishes and links into the project are its
  // own, not project inputs: they are neither dependencies nor listings.
  const published = (p) => [...links].some(([link, target]) =>
    [link, target].some((root) => p === root || p.startsWith(root + "/")));
  // What ttc writes into an output directory is never a project input, as
  // `tsc` leaves its own outputs out of a default `include`: no glob finds
  // them, while `files` entries and imports still resolve there. Those are
  // the declaration sidecars it names after their sources (`x.tt.d.ts`,
  // `x.ttx.d.ts`, each with its `.map`) and the support package it owns at
  // the output root's `tt/`. The directory may also hold the sources
  // themselves (sidecars beside them), which stay inputs.
  const outputRoot = (d) => {
    let real = d;
    try { real = fs.realpathSync(d); } catch {}
    real = real.replaceAll("\\", "/");
    const root = outputs.find((dir) => real === dir || real.startsWith(dir + "/"));
    return root === undefined ? undefined : { root, real };
  };
  const writtenByTtc = (name) => /\.ttx?\.d\.ts(\.map)?$/.test(name);
  const realDirectories = new Map();
  const served = (f) => {
    if (files.has(f)) return f;
    const dir = path.dirname(f);
    if (!realDirectories.has(dir)) {
      let real = null;
      try { real = fs.realpathSync(dir).replaceAll("\\", "/"); } catch {}
      realDirectories.set(dir, real);
    }
    const realDir = realDirectories.get(dir);
    if (realDir !== null && realDir !== dir) {
      const real = path.join(realDir, path.basename(f));
      if (files.has(real)) return real;
    }
    if (!LOWERED.test(f)) return undefined;
    const extension = path.extname(f);
    const source = f.slice(0, -extension.length);
    let realSource;
    try { realSource = fs.realpathSync(source).replaceAll("\\", "/"); } catch { return undefined; }
    const real = realSource + extension;
    return realSource !== source && files.has(real) ? real : undefined;
  };
  return {
    // A `.tt` source the engine did not serve does not exist for TypeScript:
    // its text is tt, not the lowered module.
    fileExists: (f) => (served(f) !== undefined ? true : TT_SOURCE.test(f) ? false : undefined),
    // `undefined` falls back to the real disk; `null` would mean "absent".
    readFile: (f) => {
      const real = served(f);
      if (real === undefined && !dependencies.has(f) && !published(f)) dependencies.set(f, diskVersion(f));
      if (configFiles.has(f)) return configFiles.get(f);
      if (real !== undefined) return files.get(real);
      return TT_SOURCE.test(f) ? null : undefined;
    },
    directoryExists: (d) => (dirs.has(d) ? true : undefined),
    realpath: (p) => {
      for (const [link, target] of links) {
        if (p === link || p.startsWith(link + "/")) return target + p.slice(link.length);
      }
      return served(p);
    },
    getAccessibleEntries: (d) => {
      const inOutput = outputRoot(d);
      let real = { files: [], directories: [] };
      try {
        for (const e of fs.readdirSync(d, { withFileTypes: true })) {
          if (e.isDirectory()) {
            if (!(inOutput && inOutput.real === inOutput.root && e.name === "tt")) {
              real.directories.push(e.name);
            }
          } else if (!(inOutput && writtenByTtc(e.name))) {
            real.files.push(e.name);
          }
        }
      } catch {
        if (!dirs.has(d)) return undefined;
      }
      if (!published(d)) listings.set(d, new Set([...real.files, ...real.directories]));
      if (!published(d) && real.files.length > 0) {
        const owned = ownedOutputs(real.files.map((name) => path.join(d, name)));
        real.files = real.files.filter((name) => !owned.has(path.join(d, name)));
      }
      const here = [...files.keys()].filter((f) => path.dirname(f) === d && !aliases.has(f));
      const names = new Set(real.files.map((f) => f));
      for (const f of here) {
        const base = path.basename(f);
        if (!names.has(base)) real.files.push(base);
      }
      // The sources ttc did not serve are not TypeScript; hide them so no
      // tool tries to read `.tt` as TypeScript.
      real.files = real.files.filter((f) => !TT_SOURCE.test(f) || files.has(path.join(d, f)));
      return real;
    },
  };
}

function fail(code, message) {
  process.stderr.write(message + "\n");
  process.exit(code);
}

async function main() {
  const readLine = lineReader();
  let open;
  try {
    open = JSON.parse(readLine());
  } catch (e) {
    fail(3, "ttc host: malformed open request: " + e.message);
  }

  let API;
  let SymbolFlags;
  let TypeFlags;
  let NodeBuilderFlags;
  let ObjectFlags;
  let isExpression;
  let isIdentifier;
  let isVariableDeclaration;
  let isBinaryExpression;
  let isStatement;
  let SyntaxKind;
  try {
    ({ API, SymbolFlags, TypeFlags, NodeBuilderFlags, ObjectFlags } = await import(open.apiModule));
    ({
      isExpression,
      isIdentifier,
      isVariableDeclaration,
      isBinaryExpression,
      isStatement,
      SyntaxKind,
    } = await import(path.resolve(path.dirname(open.apiModule), "../../ast/index.js")));
  } catch (e) {
    fail(2, "ttc host: cannot load the TypeScript API from " + open.apiModule + ": " + e.message);
  }

  // The modules ttc serves, mutated in place between requests: the file
  // system the compiler holds is this one, so an updated text is visible as
  // soon as the snapshot is told the file changed.
  const files = new Map();
  const dirs = new Set();
  const configFiles = new Map();
  const configSpans = new Map();
  const dependencies = new Map();
  const listings = new Map();
  const links = new Map();
  const aliases = new Set();
  const outputs = (open.outputs ?? []).map((dir) => path.resolve(dir).replaceAll("\\", "/"));
  const pendingDisk = { created: [], changed: [], deleted: [] };
  let diskGeneration = 0;
  let mapped = false;
  let carried = false;
  const mapperPackage = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    `typed-engine-mapper-${createHash("sha256").update(process.execPath).digest("hex").slice(0, 16)}`,
  );
  // The client runs the executable shipped beside it — the one it was
  // built against, and the same one ttc drives as a language server.
  const connect = () => new API({
    cwd: open.cwd,
    runExternalCode: mapped,
    fs: layeredFileSystem(files, aliases, dirs, configFiles, dependencies, listings, links, outputs, ownedOutputs),
  });
  function ownedOutputs(paths) {
    writeLine(JSON.stringify({ ownedOutputs: paths }));
    const line = readLine();
    if (line === null) throw new Error("ttc closed the session while the host asked about its outputs");
    return new Set(JSON.parse(line).owned);
  }
  let api = connect();
  writeLine(JSON.stringify({ ok: true }));

  let opened = false;
  const openRoots = new Set();
  const snapshots = [];
  const updateSnapshot = (params) => {
    const snapshot = api.updateSnapshot(params);
    snapshots.push(snapshot);
    return snapshot;
  };
  try {
    while (true) {
      const line = readLine();
      if (line === null) break;
      let answer;
      try {
        if (process.env.TTC_TYPESCRIPT_BACKEND_FAIL_FOR_TEST === "1") {
          throw new Error("injected TypeScript backend contract failure");
        }
        const job = JSON.parse(line);
        if (job.diskGeneration) {
          answer = detectDisk();
        } else if (job.configuredMappers) {
          answer = configuredMappers();
        } else {
          try {
            answer = handle(job);
          } catch (e) {
            reconnect();
            throw e;
          }
        }
      } catch (e) {
        // The Rust boundary classifies this as an internal compiler error.
        // Keep the protocol response actionable without leaking a Node
        // implementation stack into the user's diagnostic stream.
        answer = { error: e instanceof Error ? e.message : String(e) };
      }
      writeLine(JSON.stringify(answer));
      try {
        const latest = snapshots.pop();
        for (const snapshot of snapshots.splice(0)) snapshot.dispose();
        if (latest) snapshots.push(latest);
      } catch {
        reconnect();
      }
    }
  } finally {
    api.close();
  }

  function detectDisk() {
    let found = false;
    for (const [file, previous] of dependencies) {
      const current = diskVersion(file);
      if (current !== previous) {
        (current === null ? pendingDisk.deleted : previous === null ? pendingDisk.created : pendingDisk.changed).push(file);
        dependencies.set(file, current);
        found = true;
      }
    }
    for (const [directory, previous] of listings) {
      let current;
      try { current = new Set(fs.readdirSync(directory)); } catch { current = new Set(); }
      for (const name of current) if (!previous.has(name)) { pendingDisk.created.push(path.join(directory, name)); found = true; }
      for (const name of previous) if (!current.has(name)) { pendingDisk.deleted.push(path.join(directory, name)); found = true; }
      listings.set(directory, current);
    }
    if (found) diskGeneration += 1;
    return { diskGeneration };
  }

  function configuredMappers() {
    if (!open.tsconfig) return { contentMappers: [] };
    const served = new Map(configFiles);
    configFiles.clear();
    try {
      const mappers = api.parseConfigFile(open.tsconfig)?.raw?.contentMappers;
      return { contentMappers: Array.isArray(mappers) ? mappers : [] };
    } finally {
      for (const [file, text] of served) configFiles.set(file, text);
    }
  }

  /** The name a module is served under in the current arrangement. */
  function served(file) {
    return mapped && LOWERED.test(file) ? file.slice(0, file.lastIndexOf(".")) : file;
  }

  /** The engine's name for a file TypeScript reported. */
  function moduleName(file) {
    if (!mapped || typeof file !== "string" || !TT_SOURCE.test(file)) return file;
    return file + (file.endsWith(".ttx") ? ".tsx" : ".ts");
  }

  /**
   * The files a job's modules are served as. Mapped, a lowered module is
   * also served under the engine's own `x.tt.ts` name, unlisted, for a root
   * outside the configuration: its inferred project names no content mapper
   * and reaches `"./y.tt"` as `y.tt.ts`, exactly as before.
   */
  function servedModules(modules) {
    aliases.clear();
    const out = [];
    for (const module of modules) {
      out.push({ ...module, path: served(module.path) });
      if (served(module.path) !== module.path) {
        aliases.add(module.path);
        out.push(module);
      }
    }
    return out;
  }

  /** The job's questions, each addressed to the file TypeScript holds. */
  function addressed(job, name) {
    const module = (entry) => ({ ...entry, module: name(entry.module) });
    return {
      ...job,
      editorDiagnostics: job.editorDiagnostics ? name(job.editorDiagnostics) : null,
      diagnosticsScope: job.diagnosticsScope ? name(job.diagnosticsScope) : null,
      referenceClosure: job.referenceClosure ? name(job.referenceClosure) : null,
      literalChecks: (job.literalChecks ?? []).map(module),
      tagChecks: (job.tagChecks ?? []).map(module),
      symbolChecks: (job.symbolChecks ?? []).map(module),
      resultShapeChecks: (job.resultShapeChecks ?? []).map(module),
      contextualSlots: (job.contextualSlots ?? []).map(module),
    };
  }

  function engineAnswer(out) {
    for (const diagnostic of [...out.diagnostics, ...out.editorDiagnostics]) {
      diagnostic.file = moduleName(diagnostic.file);
      for (const related of diagnostic.related ?? []) related.file = moduleName(related.file);
      if (diagnostic.mismatch?.declaration) {
        diagnostic.mismatch.declaration.file = moduleName(diagnostic.mismatch.declaration.file);
      }
    }
    out.projectModules = out.projectModules.map(moduleName);
    return out;
  }

  /** Whether the configuration names a content mapper for anything but tt. */
  function foreignMappers(parsed) {
    const mappers = parsed?.raw?.contentMappers;
    return Array.isArray(mappers) && mappers.some((entry) =>
      !entry || typeof entry !== "object" || !Array.isArray(entry.extensions) ||
      entry.extensions.some((extension) => extension !== ".tt" && extension !== ".ttx"));
  }

  /** Switches the arrangement: a fresh compiler, the project not yet open. */
  function reconnect() {
    snapshots.length = 0;
    try {
      api.close();
    } catch {}
    links.clear();
    if (mapped) {
      const link = path.join(path.dirname(open.tsconfig), "node_modules", MAPPER_PACKAGE);
      fs.mkdirSync(mapperPackage, { recursive: true });
      publishFile(path.join(mapperPackage, "mapper.cjs"), IDENTITY_MAPPER);
      publishFile(path.join(mapperPackage, "package.json"), JSON.stringify({
        name: MAPPER_PACKAGE,
        version: "0.0.0",
        typescript: { contentMapper: { exec: [process.execPath, path.join(mapperPackage, "mapper.cjs")] } },
      }));
      links.set(link, mapperPackage);
      for (let d = link; d !== path.dirname(d); d = path.dirname(d)) dirs.add(d);
    }
    api = connect();
    opened = false;
    openRoots.clear();
  }

  /** One `ask`: refresh the served modules, then answer every question. */
  function handle(request) {
    let job = request;
    const out = {
      projectModules: [],
      diagnostics: [],
      editorDiagnostics: [],
      projectDiagnostics: [],
      literalMissing: [],
      tagMissing: [],
      tagMembers: [],
      symbols: [],
      resultShapes: [],
      declarations: [],
      contextualSlots: [],
      contextualRoundTrips: 0,
    };
    let changes = serve(files, dirs, servedModules(job.modules ?? []));
    detectDisk();
    for (const kind of ["created", "changed", "deleted"]) {
      changes[kind].push(...pendingDisk[kind]);
      pendingDisk[kind] = [];
    }
    out.diskGeneration = diskGeneration;
    if (open.tsconfig) {
      // Parse JSONC and discover extended configurations through TypeScript.
      // Translate the schema's file patterns through the same path projection
      // as modules; never alter source strings or infer membership from a scan.
      const previous = new Map(configFiles);
      configFiles.clear();
      configSpans.clear();
      // A configuration TypeScript cannot read is TS5083, the diagnostic
      // `tsc` reports for it. No project exists until it can be read again,
      // and then it is opened afresh.
      const unreadable = api.readConfigFile(open.tsconfig).error;
      if (unreadable?.code === CANNOT_READ_FILE) {
        if (opened) reconnect();
        out.projectDiagnostics.push({ file: open.tsconfig, code: unreadable.code, message: unreadable.text });
        out.dependencies = [...dependencies.keys()];
        out.directories = [...listings.keys()];
        return engineAnswer(out);
      }
      const parsed = api.parseConfigFile(open.tsconfig);
      const circular = (parsed?.errors ?? []).filter((d) => d.code === CIRCULAR_CONFIGURATION);
      if (circular.length > 0) {
        if (opened) reconnect();
        for (const d of circular) {
          out.projectDiagnostics.push({ file: open.tsconfig, code: d.code, message: messageText(d) });
        }
        out.dependencies = [...dependencies.keys()];
        out.directories = [...listings.keys()];
        return engineAnswer(out);
      }
      const wanted = !foreignMappers(parsed);
      if (wanted !== mapped) {
        mapped = wanted;
        reconnect();
        changes = serve(files, dirs, servedModules(job.modules ?? []));
        api.parseConfigFile(open.tsconfig);
      }
      for (const file of new Set([open.tsconfig, ...dependencies.keys()])) {
        if (!file.endsWith(".json") || !(files.has(file) || fs.existsSync(file))) continue;
        const { config } = api.readConfigFile(file);
        if (!config || typeof config !== "object") continue;
        const text = files.has(file) ? files.get(file) : fs.readFileSync(file, "utf8");
        const tree = jsoncTree(text);
        if (tree?.kind !== "object" && tree?.kind !== "empty") continue;
        const member = (key) => tree.members.findLast((entry) => entry.key === key);
        const edits = [];
        // Unmapped, a pattern naming `.tt` names the lowered `.tt.ts`. Mapped,
        // user patterns already name what TypeScript sees; only a
        // configuration the engine serves names its modules by the engine's
        // `x.tt.ts`.
        const rename = mapped
          ? (files.has(file) ? served : null)
          : (entry) => entry + (entry.endsWith(".ttx") ? ".tsx" : entry.endsWith(".tt") ? ".ts" : "");
        for (const key of rename ? ["files", "include", "exclude"] : []) {
          const list = member(key)?.value;
          if (!Array.isArray(config[key]) || list?.kind !== "array") continue;
          config[key].forEach((entry, index) => {
            const element = list.elements[index];
            if (typeof entry !== "string" || element?.kind !== "string") return;
            const renamed = rename(entry);
            if (renamed !== entry) edits.push({ start: element.start, end: element.end, text: JSON.stringify(renamed) });
          });
        }
        const mappers = member("contentMappers");
        if (mapped && path.resolve(file) === path.resolve(open.tsconfig)) {
          const own = JSON.stringify([{ package: MAPPER_PACKAGE, extensions: [".tt", ".ttx"] }]);
          edits.push(mappers
            ? { start: mappers.value.start, end: mappers.value.end, text: own }
            : tree.kind === "empty"
              ? { start: tree.start, end: tree.start, text: `{"contentMappers":${own}}` }
              : { start: tree.start + 1, end: tree.start + 1, text: `"contentMappers":${own}${tree.members.length > 0 ? "," : ""}` });
        } else if (Array.isArray(config.contentMappers) && mappers) {
          let filtered = false;
          const kept = config.contentMappers
            .map(entry => {
              if (!entry || typeof entry !== "object" || !Array.isArray(entry.extensions)) return entry;
              const extensions = entry.extensions.filter(extension => extension !== ".tt" && extension !== ".ttx");
              if (extensions.length === entry.extensions.length) return entry;
              filtered = true;
              return extensions.length > 0 ? { ...entry, extensions } : null;
            })
            .filter(entry => entry !== null);
          if (filtered && kept.length > 0) {
            edits.push({ start: mappers.value.start, end: mappers.value.end, text: JSON.stringify(kept) });
          } else if (filtered) {
            edits.push(mappers.comma !== null
              ? { start: mappers.start, end: mappers.comma + 1, text: "" }
              : { start: mappers.previousComma ?? mappers.start, end: mappers.end, text: "" });
          }
        }
        if (edits.length > 0) {
          const edited = editedText(text, edits);
          configFiles.set(file, edited.text);
          configSpans.set(file, edited.spans);
        }
      }
      const carries = mapped && configFiles.has(open.tsconfig);
      if (opened && carries !== carried) reconnect();
      carried = carries;
      for (const file of new Set([...previous.keys(), ...configFiles.keys()])) {
        if (previous.get(file) !== configFiles.get(file)) changes.changed.push(file);
      }
    }
    // With a `tsconfig.json` the project is the user's own. Without one —
    // a workspace that never configured TypeScript — the modules are opened
    // directly and the compiler infers a project for them, which is what an
    // editor does for a loose file.
    //
    // Opening happens once; from then on the snapshot is only told what
    // changed, which is the whole point of keeping this process alive.
    const paths = (job.modules ?? []).map((m) => m.path);
    // Without a configuration the project is whatever is opened, so the
    // hand-written `.ts` files come along: one nothing imports is still the
    // user's code, and `ttc --types src` is expected to check it.
    const params = opened
      ? { fileChanges: changes, ...(!open.tsconfig ? { openFiles: [...paths, ...(job.sources ?? [])] } : {}) }
      : open.tsconfig
        ? { openProjects: [open.tsconfig] }
        : { openFiles: [...paths, ...(job.sources ?? [])] };
    let snapshot = updateSnapshot(params);
    let project = open.tsconfig
      ? snapshot.getProject(open.tsconfig)
      : paths.map((p) => snapshot.getDefaultProjectForFile(p)).find(Boolean);
    if (!project) {
      throw new Error("no project for " + (open.tsconfig ?? paths[0] ?? "<nothing>"));
    }
    opened = true;

    // The candidate modules come from a filesystem scan so the layered
    // filesystem can implement tsconfig globs and module resolution. The
    // configured program is the authority on which candidates actually
    // belong. Keep that distinction explicit: a file outside `include`
    // may still join through an import, while an unrelated file must never
    // receive a checker position query.
    const projectModules = new Set(
      paths.map(served).filter((module) => project.program.getSourceFile(module) !== undefined),
    );

    const outside = open.tsconfig
      ? (job.roots ?? []).filter((root) => files.has(root) && !projectModules.has(served(root)))
      : [];
    const opening = outside.filter((root) => !openRoots.has(root));
    const closing = [...openRoots].filter((root) => !outside.includes(root));
    if (opening.length > 0 || closing.length > 0) {
      snapshot = updateSnapshot({ openFiles: opening, closeFiles: closing });
      for (const root of closing) openRoots.delete(root);
      for (const root of opening) openRoots.add(root);
      project = snapshot.getProject(open.tsconfig);
      if (!project) throw new Error("no project for " + open.tsconfig);
    }
    const groups = [{ project, members: projectModules, whole: true }];
    for (const root of outside) {
      const owner = snapshot.getDefaultProjectForFile(root);
      if (!owner || owner.id === project.id || !owner.program.getSourceFile(root)) continue;
      const group = groups.find((candidate) => candidate.project.id === owner.id);
      if (group) group.members.add(root);
      else groups.push({ project: owner, members: new Set([root]), whole: false });
    }
    out.projectModules = groups.flatMap((group) => [...group.members]);
    const names = new Map();
    for (const group of groups) {
      for (const member of group.members) names.set(group.whole ? moduleName(member) : member, member);
    }
    job = addressed(job, (module) => names.get(module) ?? served(module));

    // The files a module's types can depend on, by TypeScript's own rules
    // (`builderState.ts`): every source file a module reference's symbol is
    // declared as, every triple-slash reference, followed transitively, from
    // the roots and from every file that affects the global scope. An
    // ambient module's declarations are in files that affect the global
    // scope, so those files are roots already.
    if (job.referenceClosure) {
      const identity = api.getCanonicalFileName(job.referenceClosure);
      const owner = groups.find(({ members }) => [...members].some((member) => api.getCanonicalFileName(member) === identity)) ?? groups[0];
      const closure = referenceClosure(owner.project, job.referenceClosure, (name) => api.getCanonicalFileName(name));
      out.referenceClosure = { files: closure.files.map(moduleName), global: closure.global.map(moduleName) };
      out.dependencies = [...dependencies.keys()];
      out.directories = [...listings.keys()];
      return engineAnswer(out);
    }

    // The native API retains virtual offsets, category, tags and related
    // places. LSP content-map conversion irreversibly merges generated spans;
    // the engine must apply its own lowering provenance before that boundary.
    if (job.editorDiagnostics) {
      const target = job.editorDiagnostics;
      const identity = api.getCanonicalFileName(target);
      const owner = groups.find(({ members }) => [...members].some((member) => api.getCanonicalFileName(member) === identity));
      if (!owner) throw new Error("no diagnostic project for " + target);
      const program = owner.project.program;
      const diagnostics = [
        ...program.getSyntacticDiagnostics(target),
        ...program.getSemanticDiagnostics(target),
        ...program.getSuggestionDiagnostics(target),
        ...(program.getCompilerOptions().declaration || program.getCompilerOptions().composite
          ? program.getDeclarationDiagnostics(target) : []),
      ];
      out.editorDiagnostics = diagnostics.map((d) => ({
        file: d.fileName, start: d.pos, end: d.end, code: d.code,
        message: messageText(d), category: d.category,
        unnecessary: d.reportsUnnecessary === true,
        deprecated: d.reportsDeprecated === true,
        related: relatedPlaces(d),
      }));
      out.dependencies = [...dependencies.keys()];
      out.directories = [...listings.keys()];
      return engineAnswer(out);
    }

    const contextual = ({ project, members }) => {
      const checker = memoizedChecker(countedChecker(project.checker, () => {
        out.contextualRoundTrips += 1;
      }));
      // Declaration handles use the API's canonical spelling, while input
      // modules retain their authored spelling. Membership is file identity,
      // including when a dependency is followed through another declaration.
      const memberIdentities = new Set([...members].map((member) => api.getCanonicalFileName(member)));
      const ownsModule = (module) => memberIdentities.has(api.getCanonicalFileName(module));
      // The storage the lowering declared in each module, annotated or not,
      // and the consts that carry values to detached storage. TypeScript
      // can name a type after it (a class expression assigned to it is
      // `typeof $tt_v0`), and an annotation that did would read the
      // compiler's glue, or itself (TS2502).
      const storage = new Map();
      const storageOf = (module, source) => {
        let symbols = storage.get(module);
        if (symbols) return symbols;
        const ends = new Set((job.contextualSlots ?? [])
          .filter((slot) => slot.module === module)
          .map((slot) => slot.declarationEnd));
        symbols = new Set();
        const names = [];
        walkTree(source, (node) => {
          if (isVariableDeclaration(node) && isIdentifier(node.name) && ends.has(node.name.end)) {
            names.push(node.name);
          }
        });
        for (const declared of checker.getSymbolAtLocation(names)) {
          if (declared) symbols.add(declared.id);
        }
        storage.set(module, symbols);
        return symbols;
      };
      // The storage no round has settled yet, and whether a node reads it,
      // directly or through a declaration whose inferred type is computed
      // from it: an unannotated variable's initializer, an unannotated
      // function's body, and for an unannotated parameter the statement
      // whose context types it. Only the lowered modules declare storage,
      // so only their declarations are followed.
      let pending;
      const pendingStorage = () => {
        if (pending) return pending;
        pending = new Set();
        const ends = new Map();
        for (const slot of job.contextualSlots ?? []) {
          if (slot.settled || !ownsModule(slot.module)) continue;
          if (!ends.has(slot.module)) ends.set(slot.module, new Set());
          ends.get(slot.module).add(slot.declarationEnd);
        }
        for (const [module, declared] of ends) {
          const source = project.program.getSourceFile(module);
          if (!source) continue;
          const names = [];
          walkTree(source, (node) => {
            if (isVariableDeclaration(node) && isIdentifier(node.name) && declared.has(node.name.end)) {
              names.push(node.name);
            }
          });
          for (const symbol of checker.getSymbolAtLocation(names)) {
            if (symbol) pending.add(symbol.id);
          }
        }
        return pending;
      };
      const readsPending = (node, own) => {
        const followed = new Set([own]);
        const reads = (node) => {
          const names = [];
          walkTree(node, (child) => {
            if (isIdentifier(child)) names.push(child);
          });
          if (!names.length) return false;
          for (let symbol of checker.getSymbolAtLocation(names)) {
            if (!symbol) continue;
            if (symbol.flags & SymbolFlags.Alias) symbol = checker.getAliasedSymbol(symbol);
            if (followed.has(symbol.id)) continue;
            followed.add(symbol.id);
            if (pendingStorage().has(symbol.id)) return true;
            for (const handle of symbol.declarations ?? []) {
              if (!ownsModule(String(handle.path))) continue;
              let declaration = handle.resolve(project);
              if (declaration?.kind === SyntaxKind.Parameter && !declaration.type) {
                while (declaration.parent && !isStatement(declaration)) declaration = declaration.parent;
              }
              if (declaration && reads(declaration)) return true;
            }
          }
          return false;
        };
        return reads(node);
      };
      const writesAny = (node) => walkTree(node, (child) => child.kind === SyntaxKind.AnyKeyword || undefined);
      const syntaxIndexes = new Map();
      const syntaxOf = (module, source) => {
        let syntax = syntaxIndexes.get(module);
        if (syntax) return syntax;
        syntax = { declarations: new Map(), identifiers: new Map(), assignments: new Map() };
        const add = (map, key, node) => {
          const nodes = map.get(key);
          if (nodes) nodes.push(node);
          else map.set(key, [node]);
        };
        walkTree(source, (node) => {
          if (isVariableDeclaration(node) && isIdentifier(node.name)) syntax.declarations.set(node.name.end, node);
          if (isIdentifier(node)) add(syntax.identifiers, node.text, node);
          if (isBinaryExpression(node) && node.operatorToken.kind === SyntaxKind.EqualsToken && isIdentifier(node.left)) {
            add(syntax.assignments, node.left.text, node);
          }
        });
        syntaxIndexes.set(module, syntax);
        return syntax;
      };
      const entries = [...(job.contextualSlots ?? []).entries()];
      const names = [];
      const reads = [];
      for (const [, slot] of entries) {
        if (slot.settled || !ownsModule(slot.module)) continue;
        const source = project.program.getSourceFile(slot.module);
        if (!source) continue;
        const syntax = syntaxOf(slot.module, source);
        const declaration = syntax.declarations.get(slot.declarationEnd);
        if (!declaration) continue;
        names.push(...(syntax.identifiers.get(declaration.name.text) ?? []));
        for (const assignment of syntax.assignments.get(declaration.name.text) ?? []) {
          names.push(assignment.left);
          reads.push(assignment.right);
        }
        if (declaration.initializer) reads.push(declaration.name, declaration.initializer);
      }
      checker.getSymbolAtLocation(names);
      checker.getTypeAtLocation(reads);
      let operandsSettling = false;
      for (const [index, slot] of [...entries.filter(([, slot]) => slot.operand), ...entries.filter(([, slot]) => !slot.operand)]) {
        if (slot.settled || !ownsModule(slot.module)) continue;
        const source = project.program.getSourceFile(slot.module);
        if (!source) continue;
        const syntax = syntaxOf(slot.module, source);
        const declaration = syntax.declarations.get(slot.declarationEnd);
        if (!declaration || (declaration.type && !slot.asserted)) continue;
        const identifiers = syntax.identifiers.get(declaration.name.text) ?? [];
        const assignments = syntax.assignments.get(declaration.name.text) ?? [];
        const symbol = checker.getSymbolAtLocation(declaration.name);
        if (!symbol) continue;
        const declaredType = declaration.initializer ? checker.getTypeAtLocation(declaration.name) : undefined;
        // The annotation is written at the storage declaration, which may
        // enclose the scope the type was observed in (a class declared in a
        // match arm's block is out of scope there) or sit in a scope where
        // another declaration of the same name shadows the one the type
        // refers to. An annotation is written only when the node denotes
        // the type at the declaration: every name it references resolves
        // there to the symbol the type itself refers to, and none to
        // storage the lowering declared. It is written without truncation:
        // a truncated type is not the type (`... 3 more ...` is not even
        // TypeScript).
        const generated = storageOf(slot.module, source);
        const annotation = (type) => {
          const node = typeNode(checker, type, declaration, NodeBuilderFlags.NoTruncation);
          return node && denotes(checker, node, type, declaration, generated, { SyntaxKind, SymbolFlags, TypeFlags })
            ? node : undefined;
        };
        const incomingOf = () => assignments.filter(assignment =>
          assignment.left.text === declaration.name.text &&
          checker.getSymbolAtLocation(assignment.left)?.id === symbol.id);
        const joinOf = (types) => types.flatMap((type, index) => types.some((other, otherIndex) =>
          index !== otherIndex && checker.isTypeAssignableTo(type, other) &&
          (!checker.isTypeAssignableTo(other, type) || otherIndex < index)) ? [] : [index]);
        const indefinite = (type) => (type.flags & (TypeFlags.Any | TypeFlags.Unknown)) || type.isErrorType();
        if (declaration.initializer && !declaration.type) {
          const captured = checker.getTypeAtLocation(declaration.initializer);
          const node = (captured.flags & TypeFlags.UniqueESSymbol) && declaredType && captured.id !== declaredType.id
            ? annotation(captured) : undefined;
          if (node) {
            out.contextualSlots.push({ index, inferred: true, annotation: project.emitter.printNode(node) });
            continue;
          }
        }
        if (slot.asserted) {
          if (!job.inferJoinTypes || operandsSettling || !declaration.type || declaration.initializer) continue;
          const context = checker.getTypeFromTypeNode(declaration.type);
          const incoming = incomingOf();
          if (!incoming.length) continue;
          const types = incoming.flatMap(assignment =>
            widenedIn(checker, checker.getTypeAtLocation(assignment.right), context, TypeFlags,
              freshLiterals(checker, assignment.right, SyntaxKind, TypeFlags)));
          const cleared = { index, inferred: true, annotation: null };
          if (types.some(indefinite)) { out.contextualSlots.push(cleared); continue; }
          const annotations = joinOf(types).map(index => annotation(types[index]));
          if (!annotations.length || !annotations.every(Boolean)) { out.contextualSlots.push(cleared); continue; }
          if (annotations.some(writesAny) && incoming.some((assignment) => readsPending(assignment.right, symbol.id))) continue;
          const texts = annotations.map((node) => project.emitter.printNode(node));
          out.contextualSlots.push({ index, inferred: true, annotation: texts.length === 1
            ? texts[0] : texts.map(t => `(${t})`).join(" | ") });
          continue;
        }
        if (slot.operand) {
          if (!job.inferJoinTypes || declaration.initializer) continue;
          const incoming = incomingOf();
          if (incoming.length !== 1) continue;
          const type = checker.getWidenedType(checker.getTypeAtLocation(incoming[0].right));
          if ((type.flags & (TypeFlags.Any | TypeFlags.Unknown)) || type.isErrorType()) continue;
          const node = annotation(type);
          if (!node || (writesAny(node) && readsPending(incoming[0].right, symbol.id))) continue;
          operandsSettling = true;
          out.contextualSlots.push({ index, inferred: true, annotation: project.emitter.printNode(node) });
          continue;
        }
        let expected;
        let ambiguous = false;
        let deferred = false;
        let provisional = true;
        for (const identifier of identifiers) {
          if (identifier === declaration.name || identifier.text !== declaration.name.text) continue;
          if (checker.getSymbolAtLocation(identifier)?.id !== symbol.id) continue;
          // A use narrowed by control flow cannot supply the declaration's
          // type: doing so would reject the initializer's other constituents.
          if (declaredType && checker.getTypeAtLocation(identifier).id !== declaredType.id) continue;
          if (impliedByBindingPattern(identifier, SyntaxKind)) continue;
          if (namesJsxTag(identifier, SyntaxKind)) continue;
          if (spreadsProperties(identifier, SyntaxKind)) continue;
          const logical = logicalLeftOperand(identifier, SyntaxKind);
          if (logical && readsPending(logical, symbol.id)) { deferred = true; break; }
          if (logical && (checker.getTypeAtLocation(logical).flags & TypeFlags.Never)) continue;
          const context = checker.getContextualType(identifier);
          if (!context || (context.flags & (TypeFlags.Any | TypeFlags.Unknown)) || context.isErrorType()) continue;
          if (expected && expected.id !== context.id) { ambiguous = true; break; }
          expected = context;
          provisional &&= assertionOperand(identifier, SyntaxKind);
        }
        if (deferred) continue;
        if (job.inferJoinTypes && !operandsSettling && !expected && !ambiguous && !declaration.initializer) {
          // A statement join must have the union of its incoming value types.
          // In particular, TS's evolving-array inference at assignment sites is
          // not expression inference. Ask for each RHS type in its branch scope
          // and serialize it at the declaration; never infer from diagnostic text.
          const incoming = incomingOf();
          const join = objectLiteralJoin(checker, incoming, identifiers, declaration, symbol, TypeFlags, ObjectFlags);
          const types = join ? [join] : incoming.flatMap(assignment =>
            widenedAtMutable(checker, checker.getTypeAtLocation(assignment.right),
              freshLiterals(checker, assignment.right, SyntaxKind, TypeFlags)));
          if (!types.length || types.some(type => (type.flags & (TypeFlags.Any | TypeFlags.Unknown)) || type.isErrorType())) continue;
          // Remove constituents subsumed by another incoming type. This is the
          // checker's assignability relation, including never[] <: number[].
          const joined = joinOf(types);
          const annotations = joined.map(index => annotation(types[index]));
          if (!annotations.length || !annotations.every(Boolean)) continue;
          // An incoming value that reads storage no round has settled yet
          // is typed by that storage's `any` where it has no type of its
          // own (without `noImplicitAny`, or where an evolving variable is
          // read in a closure), and a join computed from it would keep that
          // `any` after the storage is settled. Such a join waits for a
          // later round, when its inputs are typed by their settled
          // storage.
          if (annotations.some(writesAny) && incoming.some((assignment) => readsPending(assignment.right, symbol.id))) continue;
          const texts = annotations.map((node) => project.emitter.printNode(node));
          out.contextualSlots.push({ index, inferred: true, annotation: texts.length === 1
            ? texts[0] : texts.map(t => `(${t})`).join(" | ") });
          continue;
        }
        if (!expected || ambiguous) continue;
        const node = annotation(expected);
        if (node) out.contextualSlots.push({ index, inferred: false, provisional, annotation: project.emitter.printNode(node) });
      }
    };
    for (const group of groups) contextual(group);
    if (job.contextualOnly) { out.dependencies = [...dependencies.keys()]; out.directories = [...listings.keys()]; return engineAnswer(out); }
    const reported = new Set();
    const unique = (diagnostics) => diagnostics.filter((d) => {
      const key = JSON.stringify([d.fileName ?? null, d.pos, d.end, d.code, messageText(d)]);
      if (reported.has(key)) return false;
      reported.add(key);
      return true;
    });
    // The whole configured program, not just the lowered modules: a
    // hand-written `.ts` and an `.tt` are in one project, so an error in
    // either is this run's to report. Which file it lands in decides how it
    // is positioned, and that is ttc's half. Another project answers only
    // for the requested modules it is the default project of.
    const answer = ({ project, members, whole, file = false }) => {
      const checker = project.checker;
      const program = project.program;
      const scope = whole ? undefined : [...members];
      const placed = (d) => !d.fileName || d.pos < 0
        ? null
        : configSpans.has(d.fileName)
          ? originalSpan(configSpans.get(d.fileName), d.pos, d.end)
          : { start: d.pos, end: d.end };
      const reportable = (d) => (open.tsconfig && whole) || placed(d) !== null;
      const compilerOptions = program.getCompilerOptions();
      const listFilesOnly = compilerOptions.listFilesOnly === true;
      const unparsed = new Set((job.unparsedDocuments ?? []).flatMap((module) => [module, served(module)]));
      const configuration = whole ? program.getConfigFileParsingDiagnostics() : [];
      const syntactic = program.getSyntacticDiagnostics(scope);
      const unparsable = (job.syntaxBlocked ?? []).some((module) => whole
        ? program.getSourceFile(served(module)) !== undefined
        : members.has(module) || members.has(served(module)));
      let stopped = unparsable || syntactic.some((d) => reportable(d) && !unparsed.has(d.fileName));
      const options = !stopped && whole ? program.getProgramDiagnostics() : [];
      stopped ||= options.some(reportable);
      const checked = !stopped && !listFilesOnly;
      const semanticStage = checked ? program.getSemanticDiagnostics(scope) : [];
      const lateStage = checked && whole ? program.getGlobalDiagnostics() : [];
      const ttSources = new Set((job.modules ?? []).map((module) => loweredSource(module.path)));
      const specifiers = [...(checked ? members : [])].flatMap((member) => {
        const sourceFile = program.getSourceFile(member);
        return sourceFile
          ? loweredModuleSpecifiers(sourceFile, ttSources, SyntaxKind).map((literal) => ({ sourceFile, literal }))
          : [];
      });
      const quiet = !stopped && specifiers.length === 0 && ![...semanticStage, ...lateStage].some(reportable);
      const declares = compilerOptions.declaration === true || compilerOptions.composite === true;
      const deferred = compilerOptions.noEmit === true || compilerOptions.noEmitOnError === true;
      // One file is checked as a language service checks it: its
      // declaration diagnostics with its semantic ones, whenever the options
      // declare (`services.ts` `getSemanticDiagnostics`).
      const declarationStage = !listFilesOnly && declares && (file ? checked : quiet || !deferred)
        ? program.getDeclarationDiagnostics(scope)
        : [];
      const semantic = unique([...semanticStage, ...declarationStage]);
      const late = unique(lateStage);
      for (const d of [...unique([...configuration, ...syntactic, ...options]), ...late]) {
        const span = placed(d);
        if (!span) {
          if (open.tsconfig && whole) out.projectDiagnostics.push({ file: d.fileName ?? null, code: d.code, message: messageText(d) });
          continue;
        }
        out.diagnostics.push({ file: d.fileName, start: span.start, end: span.end, code: d.code, message: messageText(d) });
      }
      for (const d of semantic) {
        if (!d.fileName) {
          if (open.tsconfig && whole) out.projectDiagnostics.push({ file: null, code: d.code, message: messageText(d) });
          continue;
        }
        const mismatch = contextualMismatch(project, checker, d, isExpression, SyntaxKind, TypeFlags);
        const receiver = lookupReceiver(project, d, SyntaxKind);
        const related = relatedPlaces(d);
        out.diagnostics.push({
          file: d.fileName,
          start: d.pos,
          end: d.end,
          code: d.code,
          message: messageText(d),
          ...(mismatch ? { mismatch } : {}),
          ...(receiver ? { receiver } : {}),
          ...(related.length > 0 ? { related } : {}),
        });
      }
      for (const { sourceFile, literal } of specifiers) {
        out.diagnostics.push({
          file: sourceFile.fileName,
          start: literal.getStart(sourceFile),
          end: literal.end,
          code: CANNOT_FIND_MODULE,
          message: `Cannot find module '${literal.text}' or its corresponding type declarations.`,
        });
      }
      /**
       * Whether a declaration lives in one of TypeScript's own lib files.
       * Answered from the program's per-file metadata when the client has it
       * — a small, cached query — rather than by fetching the whole source
       * file just to ask about it.
       */
      const isDefaultLibrary = (declaration) => {
        if (!declaration.path) return false;
        if (typeof project.program.getSourceFileMetadata === "function") {
          const metadata = project.program.getSourceFileMetadata(String(declaration.path));
          return metadata ? metadata.isDefaultLibrary === true : false;
        }
        const file = project.program.getSourceFile(String(declaration.path));
        return file ? project.program.isSourceFileDefaultLibrary(file) : false;
      };

      // The per-position questions, batched by module: the checker's position
      // APIs take an array of positions for one file, so a project's worth of
      // questions costs one round trip per module per kind instead of one per
      // question. The batch is an implementation detail of this host — the
      // job's own indices are what every answer is keyed by, and `perModule`
      // scatters each answer back onto the entry it was asked for.

      // Literal- and tag-match exhaustiveness share one type question: the
      // type TypeScript computes AT the scrutinee — narrowing included —
      // decides what the arms miss.
      const typeChecks = [
        ...(job.literalChecks ?? []).map((check, index) => ({ check, index, tag: false })),
        ...(job.tagChecks ?? []).map((check, index) => ({ check, index, tag: true })),
      ].filter((entry) => members.has(entry.check.module));
      const types = perModule(typeChecks, (module, positions) =>
        batched(
          "typesAtPositions",
          () => checker.getTypeAtPosition(module, positions),
          () => positions.map((p) => checker.getTypeAtPosition(module, p)),
        ));
      // A project's matches share their scrutinee types: one variant matched in
      // three hundred places is one type, and the answers derived from a type
      // — its constituents, each constituent's `kind` — depend on nothing
      // else. Both are asked once per type, not once per match. (Type ids are
      // snapshot-scoped, so the memo lives and dies with this ask.)
      const constituentCache = new Map();
      const constituentsOf = (type) => {
        let constituents = constituentCache.get(type.id);
        if (constituents === undefined) {
          constituents = type.isUnionType?.() ? type.getTypes() : [type];
          constituentCache.set(type.id, constituents);
        }
        return constituents;
      };
      const kindCache = new Map();
      const kindSymbolOf = (constituent) => {
        let kind = kindCache.get(constituent.id);
        if (kind === undefined) {
          kind = checker.getPropertyOfType(constituent, "kind") ?? null;
          kindCache.set(constituent.id, kind);
        }
        return kind;
      };
      // A tag check needs a second round: the `kind` property's type of every
      // constituent. The types of the distinct `kind` symbols — across every
      // tag check — are one batch.
      const tagWork = [];
      typeChecks.forEach((entry, at) => {
        if (!entry.tag) {
          const missing = missingLiterals(types[at], entry.check.covered, constituentsOf);
          if (missing) out.literalMissing.push({ index: entry.index, missing });
          return;
        }
        // A tt variant lowers to a discriminated union, so the question is
        // "which `kind` values does the scrutinee's type still allow?" —
        // again at the match, so a case an earlier guard removed is not
        // demanded back.
        const symbols = tagKindSymbols(types[at], constituentsOf, kindSymbolOf);
        if (symbols) tagWork.push({ index: entry.index, covered: entry.check.covered, symbols });
      });
      if (tagWork.length > 0) {
        // One question per distinct symbol: the same case tag reached from
        // three hundred matches is still one symbol.
        const distinct = new Map();
        for (const work of tagWork) {
          for (const symbol of work.symbols) {
            if (!distinct.has(symbol.id)) distinct.set(symbol.id, symbol);
          }
        }
        const asked = [...distinct.values()];
        const kinds = batched(
          "typesOfSymbols",
          () => checker.getTypeOfSymbol(asked),
          () => asked.map((symbol) => checker.getTypeOfSymbol(symbol)),
        );
        const valueOf = new Map(asked.map((symbol, i) => [symbol.id, literalValue(kinds[i])]));
        for (const work of tagWork) {
          const tags = work.symbols.map((symbol) => valueOf.get(symbol.id));
          // Every constituent must carry a single string-literal `kind`;
          // anything less definite makes the whole question indefinite, and
          // an indefinite question gets no answer.
          if (tags.some((tag) => typeof tag !== "string")) continue;
          const seen = new Set(work.covered);
          const missing = tags.filter((tag) => !seen.has(tag));
          if (missing.length > 0) out.tagMissing.push({ index: work.index, missing });
          // The whole member list, not just what the arms left out: tt runs
          // its own exhaustiveness algorithm over it, which is what sees
          // holes *inside* a payload (TASK-108). The `missing` above stays
          // for the answer tt falls back to.
          out.tagMembers.push({ index: work.index, tags });
        }
      }

      const resultChecks = (job.resultShapeChecks ?? [])
        .map((check, index) => ({ check, index }))
        .filter((entry) => members.has(entry.check.module));
      const resultTypes = resultChecks.map((entry) => {
        const sourceFile = project.program.getSourceFile(entry.check.module);
        if (!sourceFile) return null;
        const expression = smallestExpressionCovering(
          sourceFile,
          entry.check.start,
          entry.check.end,
          isExpression,
        );
        return expression ? checker.getTypeAtLocation(expression) : null;
      });
      resultChecks.forEach((entry, at) => {
        if (
          resultTypes[at] &&
          isDefiniteResult(resultTypes[at], constituentsOf, kindSymbolOf, checker)
        ) {
          out.resultShapes.push({ index: entry.index });
        }
      });

      // Resolution: the primitive tt's `val` is built from. Which binding an
      // identifier names, and whether a method is a built-in, are both "what
      // symbol is this?" — asked here, interpreted by tt.
      const symbolChecks = (job.symbolChecks ?? [])
        .map((check, index) => ({ check, index }))
        .filter((entry) => members.has(entry.check.module));
      const symbols = perModule(symbolChecks, (module, positions) =>
        batched(
          "symbolsAtPositions",
          () => checker.getSymbolAtPosition(module, positions),
          () => positions.map((p) => checker.getSymbolAtPosition(module, p)),
        ));
      symbolChecks.forEach((entry, at) => {
        const symbol = entry.check.binding
          ? declaredBinding(checker, symbols[at], entry.check, { SyntaxKind, SymbolFlags })
          : symbols[at];
        if (!symbol) return; // `any`, unresolved — never a verdict
        const declarations = symbol.declarations ?? [];
        out.symbols.push({
          index: entry.index,
          id: symbol.id,
          name: symbol.name,
          // Whether this is one of TypeScript's own declarations is the
          // compiler's answer, not a guess from the path: a released package
          // reads its libraries from disk while a built checkout serves them
          // from `bundled:///`, and both are the same fact.
          builtin: declarations.length > 0 && declarations.every(isDefaultLibrary),
        });
      });
    };
    if (job.diagnosticsScope) {
      // TypeScript's `semanticCheck(file)` (`server/session.ts`): the
      // diagnostics of the one file an editor shows, decided by that file's
      // own syntax, never stopped by another file's.
      const identity = api.getCanonicalFileName(job.diagnosticsScope);
      for (const group of groups) {
        const target = [...group.members].find((member) => api.getCanonicalFileName(member) === identity);
        if (target === undefined) continue;
        answer({ project: group.project, members: new Set([target]), whole: false, file: true });
        break;
      }
    } else {
      for (const group of groups) answer(group);
    }
    // Declaration emit, in memory. The compiler writes the `.d.ts` for a
    // lowered module exactly as it would for a hand-written one, so ttc
    // never generates TypeScript declaration syntax itself.
    if (job.emitDeclarations) {
      // Declaration emit is newer than the checker API: a released 7.0
      // client can check but cannot emit.
      if (typeof project.program.getDeclarationEmit !== "function") {
        process.exitCode = 5;
        fail(5, "ttc host: the resolved TypeScript has no declaration emit API");
      }
      const requested = (job.modules ?? []).map((m) => m.path);
      for (const group of groups) {
        const modules = requested
          .map((path) => (group.members.has(path) ? path : served(path)))
          .filter((module) => group.members.has(module));
        if (modules.length === 0) continue;
        for (const module of modules) {
          const emitted = group.project.program.getDeclarationEmit([module]);
          for (const [path, file] of emitted.outputFiles) {
            if (path.endsWith(".map")) continue;
            out.declarations.push({ module: moduleName(module), text: file.text });
          }
        }
      }
    }
    out.dependencies = [...dependencies.keys()];
    out.directories = [...listings.keys()];
    return engineAnswer(out);
  }
}

/**
 * `root` and the files its types can depend on: TypeScript's
 * `BuilderState.getAllDependencies` over `getReferencedFiles`, with every
 * file that affects the global scope as a further root (such a file is a
 * dependency of every file). A file affects the global scope
 * (`isFileAffectingGlobalScope`) when it is not a module, declaration files
 * included, or, conservatively, when it augments any module. A type
 * reference directive's target is not followed: the API does not expose its
 * resolution, and a local one that declares globals is a root as such.
 */
function referenceClosure(project, root, canonical) {
  const program = project.program;
  const checker = project.checker;
  const local = (name) => {
    const metadata = typeof program.getSourceFileMetadata === "function" ? program.getSourceFileMetadata(name) : undefined;
    return !(metadata && (metadata.isDefaultLibrary || metadata.isFromExternalLibrary));
  };
  const global = [];
  for (const name of program.getSourceFileNames()) {
    if (!local(name) || name.endsWith(".json")) continue;
    const sourceFile = program.getSourceFile(name);
    if (sourceFile && (!sourceFile.externalModuleIndicator || sourceFile.moduleAugmentations.length > 0)) {
      global.push(sourceFile.fileName);
    }
  }
  const seen = new Set();
  const files = [];
  const queue = [root, ...global];
  while (queue.length > 0) {
    const name = queue.pop();
    const key = canonical(name);
    if (seen.has(key)) continue;
    seen.add(key);
    const sourceFile = program.getSourceFile(name);
    if (!sourceFile) continue;
    files.push(sourceFile.fileName);
    if (!local(sourceFile.fileName)) continue;
    const literals = [...sourceFile.imports, ...sourceFile.moduleAugmentations];
    const symbols = literals.length > 0 ? checker.getSymbolAtLocation(literals) : [];
    for (const symbol of symbols) {
      for (const declaration of symbol?.declarations ?? []) {
        if (declaration.path) queue.push(String(declaration.path));
      }
    }
    for (const reference of sourceFile.referencedFiles) {
      queue.push(path.resolve(path.dirname(sourceFile.fileName), reference.fileName));
    }
  }
  return { files, global };
}

/**
 * The type node TypeScript's node builder writes for `type` at `location`,
 * or `undefined` when it cannot write one: the node builder gives up on a
 * type it cannot name there (the instance or constructor type of an
 * anonymous class), which is `typeToTypeNode`'s documented `undefined`.
 *
 * The server sends that answer as an encoded `null`, while this client
 * treats only an empty payload as no node and hands the four bytes to its
 * node decoder, which throws. The same session still answers a second
 * question about the same type, which tells that answer apart from a
 * session that stopped answering; a session failure propagates.
 */
function logicalLeftOperand(node, SyntaxKind) {
  let operand = node;
  while (operand.parent?.kind === SyntaxKind.ParenthesizedExpression) operand = operand.parent;
  const parent = operand.parent;
  if (parent?.kind !== SyntaxKind.BinaryExpression || parent.right !== operand) return undefined;
  const operator = parent.operatorToken.kind;
  return operator === SyntaxKind.BarBarToken || operator === SyntaxKind.AmpersandAmpersandToken ||
    operator === SyntaxKind.QuestionQuestionToken ? parent.left : undefined;
}

function namesJsxTag(node, SyntaxKind) {
  const parent = node.parent;
  return !!parent
    && (parent.kind === SyntaxKind.JsxOpeningElement || parent.kind === SyntaxKind.JsxSelfClosingElement)
    && parent.tagName === node;
}

function spreadsProperties(node, SyntaxKind) {
  let current = node;
  while (current.parent && current.parent.kind === SyntaxKind.ParenthesizedExpression) {
    current = current.parent;
  }
  const parent = current.parent;
  return !!parent
    && (parent.kind === SyntaxKind.SpreadAssignment || parent.kind === SyntaxKind.JsxSpreadAttribute)
    && parent.expression === current;
}

function impliedByBindingPattern(node, SyntaxKind) {
  for (let current = node; ;) {
    const parent = current.parent;
    if (!parent) return false;
    switch (parent.kind) {
      case SyntaxKind.ParenthesizedExpression:
      case SyntaxKind.ArrayLiteralExpression:
      case SyntaxKind.ObjectLiteralExpression:
        break;
      case SyntaxKind.PropertyAssignment:
        if (parent.initializer !== current) return false;
        break;
      case SyntaxKind.ConditionalExpression:
        if (parent.condition === current) return false;
        break;
      case SyntaxKind.BinaryExpression: {
        const operator = parent.operatorToken.kind;
        if (operator !== SyntaxKind.BarBarToken && operator !== SyntaxKind.QuestionQuestionToken &&
            !((operator === SyntaxKind.AmpersandAmpersandToken || operator === SyntaxKind.CommaToken) &&
              parent.right === current)) return false;
        break;
      }
      case SyntaxKind.VariableDeclaration:
        return parent.initializer === current && !parent.type &&
          (parent.name.kind === SyntaxKind.ObjectBindingPattern ||
            parent.name.kind === SyntaxKind.ArrayBindingPattern);
      default:
        return false;
    }
    current = parent;
  }
}

function assertionOperand(node, SyntaxKind) {
  let operand = node;
  while (operand.parent && (operand.parent.kind === SyntaxKind.ParenthesizedExpression ||
      operand.parent.kind === SyntaxKind.NonNullExpression)) operand = operand.parent;
  const parent = operand.parent;
  if (!parent || parent.expression !== operand) return false;
  if (parent.kind === SyntaxKind.SatisfiesExpression) return true;
  if (parent.kind !== SyntaxKind.AsExpression && parent.kind !== SyntaxKind.TypeAssertionExpression) return false;
  const type = parent.type;
  return !(type.kind === SyntaxKind.TypeReference && type.typeName.kind === SyntaxKind.Identifier &&
    type.typeName.text === "const" && !type.typeArguments);
}

function widenedIn(checker, type, context, TypeFlags, fresh) {
  const kinds = (candidate, flags) => !!(candidate.flags & flags) ||
    ((candidate.isUnionType() || candidate.isIntersectionType()) && candidate.getTypes().some((t) => kinds(t, flags)));
  const literalOf = (candidate, target) => {
    if (target.isUnionType() || target.isIntersectionType()) return target.getTypes().some((t) => literalOf(candidate, t));
    if (target.flags & TypeFlags.InstantiableNonPrimitive) {
      const constraint = checker.getBaseConstraintOfType(target);
      if (!constraint) return false;
      return kinds(constraint, TypeFlags.String) && kinds(candidate, TypeFlags.StringLiteral) ||
        kinds(constraint, TypeFlags.Number) && kinds(candidate, TypeFlags.NumberLiteral) ||
        kinds(constraint, TypeFlags.BigInt) && kinds(candidate, TypeFlags.BigIntLiteral) ||
        kinds(constraint, TypeFlags.ESSymbol) && kinds(candidate, TypeFlags.UniqueESSymbol) ||
        literalOf(candidate, constraint);
    }
    return !!(target.flags & (TypeFlags.StringLiteral | TypeFlags.Index | TypeFlags.TemplateLiteral | TypeFlags.StringMapping)) && kinds(candidate, TypeFlags.StringLiteral) ||
      !!(target.flags & TypeFlags.NumberLiteral) && kinds(candidate, TypeFlags.NumberLiteral) ||
      !!(target.flags & TypeFlags.BigIntLiteral) && kinds(candidate, TypeFlags.BigIntLiteral) ||
      !!(target.flags & TypeFlags.BooleanLiteral) && kinds(candidate, TypeFlags.BooleanLiteral) ||
      !!(target.flags & TypeFlags.UniqueESSymbol) && kinds(candidate, TypeFlags.UniqueESSymbol);
  };
  return literalOf(type, context) ? [checker.getWidenedType(type)] : widenedAtMutable(checker, type, fresh);
}

function freshLiterals(checker, node, SyntaxKind, TypeFlags) {
  const literal = TypeFlags.StringLiteral | TypeFlags.NumberLiteral | TypeFlags.BigIntLiteral |
    TypeFlags.BooleanLiteral | TypeFlags.EnumLiteral;
  const fresh = new Set();
  const addFresh = (type) => {
    if (!type) return;
    if (type.isUnionType()) { for (const constituent of type.getTypes()) addFresh(constituent); return; }
    const regular = (type.flags & literal) ? type.getRegularType() : undefined;
    if (regular && regular.id !== type.id) fresh.add(regular.id);
  };
  const addRegular = (type) => {
    if (!type) return;
    if (type.isUnionType()) { for (const constituent of type.getTypes()) addRegular(constituent); return; }
    if (type.flags & literal) fresh.add(type.id);
  };
  const pending = [node];
  while (pending.length > 0) {
    const expression = pending.pop();
    switch (expression.kind) {
      case SyntaxKind.StringLiteral:
      case SyntaxKind.NoSubstitutionTemplateLiteral:
      case SyntaxKind.NumericLiteral:
      case SyntaxKind.BigIntLiteral:
      case SyntaxKind.TrueKeyword:
      case SyntaxKind.FalseKeyword:
        addRegular(checker.getTypeAtLocation(expression));
        continue;
      case SyntaxKind.PrefixUnaryExpression:
        if ((expression.operator === SyntaxKind.MinusToken || expression.operator === SyntaxKind.PlusToken) &&
            (expression.operand.kind === SyntaxKind.NumericLiteral || expression.operand.kind === SyntaxKind.BigIntLiteral)) {
          addRegular(checker.getTypeAtLocation(expression));
        }
        continue;
      case SyntaxKind.ParenthesizedExpression:
      case SyntaxKind.SatisfiesExpression:
      case SyntaxKind.NonNullExpression:
        pending.push(expression.expression);
        continue;
      case SyntaxKind.ConditionalExpression:
        pending.push(expression.whenFalse, expression.whenTrue);
        continue;
      case SyntaxKind.BinaryExpression: {
        const operator = expression.operatorToken.kind;
        if (operator === SyntaxKind.CommaToken) pending.push(expression.right);
        if (operator === SyntaxKind.AmpersandAmpersandToken || operator === SyntaxKind.BarBarToken ||
            operator === SyntaxKind.QuestionQuestionToken) {
          pending.push(expression.right, expression.left);
        }
        continue;
      }
      case SyntaxKind.Identifier:
      case SyntaxKind.PropertyAccessExpression: {
        const name = expression.kind === SyntaxKind.Identifier ? expression : expression.name;
        const symbol = checker.getSymbolAtLocation(name);
        if (symbol) addFresh(checker.getTypeOfSymbolAtLocation(symbol, name));
        continue;
      }
      case SyntaxKind.CallExpression:
      case SyntaxKind.NewExpression:
      case SyntaxKind.TaggedTemplateExpression: {
        const signature = checker.getResolvedSignature(expression);
        if (signature) addFresh(checker.getReturnTypeOfSignature(signature));
        continue;
      }
      default:
    }
  }
  return fresh;
}

function objectLiteralJoin(checker, incoming, identifiers, declaration, symbol, TypeFlags, ObjectFlags) {
  const constituents = (type) => type.isUnionType() ? type.getTypes() : [type];
  const objectLiteral = (type) => !!(type.flags & TypeFlags.Object) && !!(type.objectFlags & ObjectFlags.ObjectLiteral);
  const assigned = incoming.flatMap((assignment) => constituents(checker.getTypeAtLocation(assignment.right)));
  if (assigned.filter(objectLiteral).length < 2) return undefined;
  const same = (a, b) => a.id === b.id || (checker.isTypeAssignableTo(a, b) && checker.isTypeAssignableTo(b, a));
  const writes = new Set(incoming.map((assignment) => assignment.left));
  for (const identifier of identifiers) {
    if (identifier === declaration.name || writes.has(identifier)) continue;
    if (checker.getSymbolAtLocation(identifier)?.id !== symbol.id) continue;
    const read = constituents(checker.getTypeAtLocation(identifier));
    if (read.every((type) => assigned.some((other) => same(type, other)))
      && assigned.every((type) => read.some((other) => same(type, other)))) {
      return checker.getWidenedType(checker.getTypeAtLocation(identifier));
    }
  }
  return undefined;
}

function widenedAtMutable(checker, type, fresh) {
  const holdsFresh = (candidate) => candidate.isUnionType()
    ? candidate.getTypes().some(holdsFresh) : fresh.has(candidate.id);
  if (type.isUnionType() && holdsFresh(type)) {
    return type.getTypes().flatMap((constituent) => widenedAtMutable(checker, constituent, fresh));
  }
  return [checker.getWidenedType(fresh.has(type.id) ? checker.getBaseTypeOfLiteralType(type) : type)];
}

function typeNode(checker, type, location, flags) {
  try {
    return checker.typeToTypeNode(type, location, flags);
  } catch (error) {
    checker.typeToString(type, location);
    return undefined;
  }
}

/**
 * Whether the node the node builder wrote for `type` denotes that type at
 * `location`. The node builder writes a symbol by its name whether or not
 * the name reaches that symbol from `location`: a type parameter or a local
 * interface shadowing another of the same name prints as the same `T` or
 * `Item`. So the node and the type are walked together, and each name the
 * node uses must resolve at `location` to the symbol of the part of the
 * type it was written for: the type parameter's own symbol, the alias or
 * declaration a type reference instantiates, the value a type query names.
 * A name that resolves to a symbol in `excluded` (generated storage) denotes
 * nothing. An `any` keyword must be written for the `any` type: the node
 * builder writes the cycle of a recursive anonymous type as `any` too. A part of the node that uses a name and cannot be paired with a
 * part of the type is not proven to denote it, so the node does not either.
 */
function denotes(checker, node, type, location, excluded, { SyntaxKind, SymbolFlags, TypeFlags }) {
  const K = SyntaxKind;
  const named = (n) => walkTree(n, (child) =>
    child.kind === K.TypeReference || child.kind === K.TypeQuery || child.kind === K.ImportType ||
    child.kind === K.ComputedPropertyName || child.kind === K.AnyKeyword || undefined);
  const aliased = (symbol) => (symbol.flags & SymbolFlags.Alias ? checker.getAliasedSymbol(symbol) : symbol);
  const entity = (name, meaning) => {
    const members = [];
    while (name.kind === K.QualifiedName) {
      members.unshift(name.right.text);
      name = name.left;
    }
    let symbol = checker.resolveName(name.text, members.length ? SymbolFlags.Namespace | SymbolFlags.Value : meaning, location);
    if (!symbol || excluded.has(symbol.id)) return undefined;
    for (const member of members) {
      const owner = aliased(symbol);
      symbol = owner.getExports().get(member.startsWith("__") ? "_" + member : member) ??
        (meaning === SymbolFlags.Value ? checker.getPropertyOfType(checker.getTypeOfSymbol(owner), member) : undefined);
      if (!symbol) return undefined;
    }
    return aliased(symbol);
  };
  const referenceArguments = (t) => {
    if (!t.isTypeReference()) return [];
    const outer = t.getTarget().getOuterTypeParameters()?.length ?? 0;
    return checker.getTypeArguments(t).slice(outer);
  };
  const constituents = (t) => (t.isUnionType() || t.isIntersectionType() ? t.getTypes() : [t]);
  const pairs = (nodes, types, scope) =>
    !nodes || nodes.every((n, i) => i < types.length && walk(n, types[i], scope));
  const optional = (n, t, question, scope) =>
    walk(n, t, scope) || (!!question && walk(n, t.getNonNullableType(), scope));
  const key = (name) =>
    name.kind === K.Identifier || name.kind === K.PrivateIdentifier || name.kind === K.StringLiteral ||
    name.kind === K.NumericLiteral ? name.text : undefined;
  const signature = (n, sig, scope) => {
    if (!sig) return false;
    const parameters = sig.getTypeParameters() ?? [];
    const declared = n.typeParameters ?? [];
    if (n.typeParameters && declared.length !== parameters.length) return false;
    if (declared.length) {
      scope = new Map(scope);
      declared.forEach((p, i) => scope.set(p.name.text, parameters[i]));
      for (const [i, p] of declared.entries()) {
        if (p.constraint && !walk(p.constraint, checker.getConstraintOfTypeParameter(parameters[i]), scope)) return false;
        if (p.default && !walk(p.default, checker.getDefaultFromTypeParameter(parameters[i]), scope)) return false;
      }
    }
    let written = [...n.parameters];
    if (written[0]?.name.kind === K.Identifier && written[0].name.text === "this") {
      const self = sig.getThisParameter();
      if (written[0].type && (!self || !walk(written[0].type, checker.getTypeOfSymbol(self), scope))) return false;
      written = written.slice(1);
    }
    const symbols = sig.getParameters();
    for (const [i, p] of written.entries()) {
      if (!p.type || !named(p.type)) continue;
      if (i >= symbols.length || !optional(p.type, checker.getTypeOfSymbol(symbols[i]), p.questionToken, scope)) return false;
    }
    if (!n.type || !named(n.type)) return true;
    if (n.type.kind === K.TypePredicate) {
      const predicate = checker.getTypePredicateOfSignature(sig);
      return !n.type.type || (!!predicate?.type && walk(n.type.type, predicate.type, scope));
    }
    return walk(n.type, sig.getReturnType(), scope);
  };
  const walk = (n, t, scope) => {
    if (!named(n)) return true;
    if (!t) return false;
    switch (n.kind) {
      case K.AnyKeyword:
        return !!(t.flags & TypeFlags.Any);
      case K.ParenthesizedType:
        return walk(n.type, t, scope);
      case K.TypeReference: {
        const local = n.typeName.kind === K.Identifier ? scope.get(n.typeName.text) : undefined;
        if (local) return t.id === local.id && !n.typeArguments;
        const symbol = entity(n.typeName, SymbolFlags.Type);
        if (!symbol) return false;
        const alias = t.getAliasSymbol();
        if (alias && alias.id === symbol.id) return pairs(n.typeArguments, t.getAliasTypeArguments(), scope);
        const own = t.getSymbol();
        return !!own && own.id === symbol.id && pairs(n.typeArguments, referenceArguments(t), scope);
      }
      case K.TypeQuery: {
        const symbol = entity(n.exprName, SymbolFlags.Value);
        const own = t.getSymbol();
        return !!symbol && !!own && own.id === symbol.id && !n.typeArguments;
      }
      case K.ImportType: {
        if (!n.qualifier) return !n.typeArguments;
        const name = n.qualifier.kind === K.QualifiedName ? n.qualifier.right.text : n.qualifier.text;
        const alias = t.getAliasSymbol();
        if (alias?.name === name) return pairs(n.typeArguments, t.getAliasTypeArguments(), scope);
        return t.getSymbol()?.name === name && pairs(n.typeArguments, referenceArguments(t), scope);
      }
      case K.UnionType:
      case K.IntersectionType:
        return n.types.every((child) => !named(child) || constituents(t).some((c) => walk(child, c, scope)));
      case K.ArrayType:
        return walk(n.elementType, referenceArguments(t)[0], scope);
      case K.TupleType: {
        const elements = referenceArguments(t);
        return n.elements.every((element, i) => {
          let written = element;
          if (written.kind === K.NamedTupleMember) written = written.type;
          if (written.kind === K.OptionalType) written = written.type;
          const rest = written.kind === K.RestType || element.dotDotDotToken;
          if (written.kind === K.RestType) written = written.type;
          if (rest && written.kind === K.ArrayType) written = written.elementType;
          return walk(written, elements[i], scope);
        });
      }
      case K.TypeOperator:
        return n.operator === K.ReadonlyKeyword && walk(n.type, t, scope);
      case K.IndexedAccessType:
        return t.isIndexedAccessType() && walk(n.objectType, t.getObjectType(), scope) &&
          walk(n.indexType, t.getIndexType(), scope);
      case K.FunctionType:
        return t.getCallSignatures().length === 1 && signature(n, t.getCallSignatures()[0], scope);
      case K.ConstructorType:
        return t.getConstructSignatures().length === 1 && signature(n, t.getConstructSignatures()[0], scope);
      case K.TypeLiteral: {
        const calls = t.getCallSignatures();
        const constructs = t.getConstructSignatures();
        const indexes = checker.getIndexInfosOfType(t);
        const seen = { call: 0, construct: 0, index: 0, methods: new Map() };
        return n.members.every((member) => {
          switch (member.kind) {
            case K.CallSignature:
              return !named(member) || signature(member, calls[seen.call++], scope);
            case K.ConstructSignature:
              return !named(member) || signature(member, constructs[seen.construct++], scope);
            case K.IndexSignature: {
              const info = indexes[seen.index++];
              return !named(member) || (!!info && walk(member.parameters[0].type, info.keyType, scope) &&
                walk(member.type, info.valueType, scope));
            }
            default: {
              if (!named(member)) return true;
              const name = key(member.name);
              const property = name === undefined ? undefined : checker.getPropertyOfType(t, name);
              if (!property) return false;
              const declared = checker.getTypeOfSymbol(property);
              if (member.kind === K.MethodSignature) {
                const index = seen.methods.get(name) ?? 0;
                seen.methods.set(name, index + 1);
                return signature(member, declared.getCallSignatures()[index], scope);
              }
              if (member.kind === K.PropertySignature || member.kind === K.GetAccessor) {
                return optional(member.type, declared, member.questionToken, scope);
              }
              if (member.kind === K.SetAccessor) return walk(member.parameters[0].type, declared, scope);
              return false;
            }
          }
        });
      }
      default:
        return false;
    }
  };
  return walk(node, type, new Map());
}

function messageText(diagnostic, level = 0) {
  let text = (level > 0 ? "\n" + "  ".repeat(level) : "") + diagnostic.text;
  for (const child of diagnostic.messageChain ?? []) text += messageText(child, level + 1);
  return text;
}

/**
 * The checker's own related places — "the expected type comes from this
 * declaration", "first declared here" — normalized to the diagnostic item
 * shape. The property names differ between clients, so both spellings are
 * accepted; an entry missing a file or a position is dropped rather than
 * guessed at.
 */
function relatedPlaces(diagnostic) {
  const entries = diagnostic.relatedInformation ?? diagnostic.related ?? [];
  const out = [];
  for (const entry of entries) {
    const file = entry.fileName ?? entry.file;
    const start = entry.pos ?? entry.start;
    const end = entry.end ?? (typeof entry.length === "number" ? start + entry.length : undefined);
    const message = entry.text ?? entry.message ?? entry.messageText;
    if (typeof file !== "string" || typeof start !== "number" || typeof end !== "number") continue;
    if (typeof message !== "string") continue;
    out.push({ file, start, end, message });
    if (out.length >= 3) break;
  }
  return out;
}

/**
 * TypeScript's diagnostics that report a source type not assignable to a
 * target type: the head messages of its assignability relation
 * (`Type '{0}' is not assignable to type '{1}'`, the argument, missing
 * property, weak type, `exactOptionalPropertyTypes` and `satisfies` forms).
 * Any other diagnostic, an arity error at an argument included, is not a
 * statement about an expression's type and its context.
 */
const ASSIGNABILITY_CODES = new Set([
  1360, 2322, 2345, 2375, 2379, 2412, 2418, 2559, 2560, 2719, 2739, 2740, 2741, 2820,
]);

/**
 * The expression an assignability diagnostic is about, with its type and
 * its contextual type. TypeScript reports the relation of an expression's
 * type to its contextual type at an error node that is either that
 * expression or stands for it: the name of the declaration, property or JSX
 * attribute it initializes, the `return` statement it is returned by, the
 * target it is assigned to, or the tag name of the JSX element its
 * attributes are passed to. The subject is found from the error node by
 * that role, never by searching the span for some expression that does not
 * fit its context. A JSX attribute is typed by its name, which TypeScript
 * gives the attribute's type and the attribute's contextual type.
 */
function contextualMismatch(project, checker, diagnostic, isExpression, SyntaxKind, TypeFlags) {
  if (!ASSIGNABILITY_CODES.has(diagnostic.code)) return null;
  const sourceFile = project.program.getSourceFile(diagnostic.fileName);
  if (!sourceFile) return null;
  const K = SyntaxKind;

  const starting = [];
  walkContaining(sourceFile, diagnostic.pos, diagnostic.end, (node) => {
    if (node.getStart(sourceFile) === diagnostic.pos) starting.push(node);
  });

  const same = (left, right) => !!left && !!right && left.kind === right.kind && left.pos === right.pos && left.end === right.end;
  const valueOf = (node) => {
    const parent = node.parent;
    if (node.kind === K.ReturnStatement) return node.expression;
    if (!parent) return undefined;
    if (parent.kind === K.JsxAttribute && same(parent.name, node)) return node;
    if (same(parent.name, node) && parent.initializer) {
      return parent.initializer;
    }
    if ((parent.kind === K.JsxOpeningElement || parent.kind === K.JsxSelfClosingElement) && same(parent.tagName, node)) {
      return parent.attributes;
    }
    if (parent.kind === K.BinaryExpression && same(parent.left, node) && parent.operatorToken.kind === K.EqualsToken) {
      return parent.right;
    }
    return undefined;
  };
  let expression;
  for (let i = starting.length - 1; i >= 0 && !expression; i--) {
    const node = starting[i];
    expression = valueOf(node) ?? (isExpression(node) && node.end === diagnostic.end ? node : undefined);
  }
  if (!expression) return null;

  let found;
  let expected;
  try {
    found = checker.getTypeAtLocation(expression);
    expected = checker.getContextualType(expression);
  } catch {
    return null;
  }
  if (!found || !expected || found.isErrorType?.() || expected.isErrorType?.()) return null;
  if (checker.isTypeAssignableTo(found, expected)) return null;
  let declaration;
  try {
    const symbol = checker.getSymbolAtPosition(
      sourceFile.fileName,
      expression.getStart(sourceFile),
    );
    const handle = symbol?.valueDeclaration ?? symbol?.declarations?.[0];
    const node = handle?.resolve?.(project);
    const file = node?.getSourceFile?.();
    if (node && file && handle) {
      declaration = {
        file: file.fileName,
        start: node.getStart(file),
        end: node.getEnd(),
      };
    }
  } catch {
    declaration = undefined;
  }
  return {
    start: expression.getStart(sourceFile),
    end: expression.getEnd(),
    expected: checker.typeToString(expected),
    found: checker.typeToString(found),
    differences: incompatibleLeaves(checker, found, expected, TypeFlags),
    ...(declaration ? { declaration } : {}),
  };
}

/**
 * TypeScript's diagnostics that say a property does not exist on a type,
 * reported at the property's name.
 */
const MISSING_PROPERTY_CODES = new Set([2339, 2551]);

/**
 * The value a missing property was looked up on: the object of the
 * property access whose name the diagnostic is at, or the value an object
 * binding pattern destructures when the name is one of its elements.
 */
function lookupReceiver(project, diagnostic, SyntaxKind) {
  if (!MISSING_PROPERTY_CODES.has(diagnostic.code)) return null;
  const sourceFile = project.program.getSourceFile(diagnostic.fileName);
  if (!sourceFile) return null;
  const K = SyntaxKind;
  let name;
  walkContaining(sourceFile, diagnostic.pos, diagnostic.end, (node) => {
    if (node.getStart(sourceFile) === diagnostic.pos && node.end === diagnostic.end) name = node;
  });
  const parent = name?.parent;
  let receiver;
  if ((parent?.kind === K.PropertyAccessExpression && parent.name === name) ||
      (parent?.kind === K.ElementAccessExpression && parent.argumentExpression === name)) {
    receiver = parent.expression;
  } else if (parent?.kind === K.BindingElement && (parent.propertyName ?? parent.name) === name &&
      parent.parent?.kind === K.ObjectBindingPattern) {
    receiver = parent.parent.parent?.initializer;
  }
  return receiver ? { start: receiver.getStart(sourceFile), end: receiver.getEnd() } : null;
}

/** The innermost expression whose source range contains the emitted value. */
function smallestExpressionCovering(sourceFile, start, end, isExpression) {
  let found = null;
  walkContaining(sourceFile, start, end, (node) => {
    if (node.getStart(sourceFile) > start) return false;
    if (isExpression(node)) found = node;
  });
  return found;
}

/**
 * The nodes whose range holds `pos..end`, outermost first and in source
 * order — what `walkTree` visits when its callback prunes every node that
 * does not hold the range, without visiting the siblings it prunes. A
 * node's children are in source order and do not overlap, so their ends do
 * not decrease and the first child that can hold the range is found by
 * binary search, as `getTokenAtPosition` finds the child holding a
 * position. The callback's `true` stops the walk and `false` skips the
 * node's children.
 */
function walkContaining(root, pos, end, enter) {
  const pending = [root];
  while (pending.length > 0) {
    const node = pending.pop();
    if (node.pos > pos || node.end < end) continue;
    const entered = enter(node);
    if (entered === true) return true;
    if (entered === false) continue;
    const children = childrenOf(node);
    let first = 0;
    let past = children.length;
    while (first < past) {
      const middle = (first + past) >>> 1;
      if (children[middle].end < end) first = middle + 1;
      else past = middle;
    }
    let last = first;
    while (last < children.length && children[last].pos <= pos) last++;
    for (let index = last - 1; index >= first; index--) pending.push(children[index]);
  }
  return false;
}

const childLists = new WeakMap();

/** A node's children in source order, listed once per node. */
function childrenOf(node) {
  let children = childLists.get(node);
  if (!children) {
    children = [];
    node.forEachChild((child) => {
      children.push(child);
    });
    childLists.set(node, children);
  }
  return children;
}

function walkTree(root, enter) {
  const pending = [root];
  while (pending.length > 0) {
    const node = pending.pop();
    const entered = enter(node);
    if (entered === true) return true;
    if (entered === false) continue;
    const children = [];
    node.forEachChild((child) => {
      children.push(child);
    });
    for (let index = children.length - 1; index >= 0; index--) pending.push(children[index]);
  }
  return false;
}

/** The `.tt`/`.ttx` source a lowered module's engine name stands for. */
function loweredSource(file) {
  return LOWERED.test(file) ? file.slice(0, file.lastIndexOf(".")) : file;
}

/**
 * The relative module specifiers of `sourceFile` that reach a served tt
 * module only through the name ttc serves it under.
 *
 * A tt module `x.tt` is served as `x.tt.ts` (`x.ttx` as `x.ttx.tsx`), and
 * TypeScript's resolution of a relative specifier appends or substitutes a
 * TypeScript extension (`./x.tt` → `x.tt.ts`; `./x.tt.js` → `x.tt.ts`, the
 * `.js` → `.ts` substitution of TypeScript's module resolution reference;
 * `./x.tt.ts` with `allowImportingTsExtensions`). Only `./x.tt` names the
 * module outside ttc: the source is `x.tt` and its output is `x.ts`, so a
 * specifier naming `x.tt.js` or `x.tt.ts` names no file on disk or in the
 * output, where TypeScript reports TS2307 for it. A file of that name that
 * does exist on disk is the user's own and is left alone.
 */
function loweredModuleSpecifiers(sourceFile, ttSources, SyntaxKind) {
  const found = [];
  const consider = (literal) => {
    if (!literal || literal.kind !== SyntaxKind.StringLiteral) return;
    const specifier = literal.text;
    if (!specifier.startsWith("./") && !specifier.startsWith("../")) return;
    const target = path.resolve(path.dirname(sourceFile.fileName), specifier);
    const match = /^(.*\.tt)\.(?:ts|js)$|^(.*\.ttx)\.(?:tsx|jsx|js)$/.exec(target);
    const source = match && (match[1] ?? match[2]);
    if (!source || !ttSources.has(source) || fs.existsSync(target)) return;
    found.push(literal);
  };
  walkTree(sourceFile, (node) => {
    if (node.kind === SyntaxKind.ImportDeclaration || node.kind === SyntaxKind.ExportDeclaration) {
      consider(node.moduleSpecifier);
    } else if (node.kind === SyntaxKind.CallExpression && node.expression.kind === SyntaxKind.ImportKeyword) {
      consider(node.arguments[0]);
    }
  });
  return found;
}

/** The union constituents of a type, or the type itself as one constituent. */
function typeConstituents(type) {
  return type.isUnionType?.() ? (type.getTypes?.() ?? [type]) : [type];
}

/** A stable structural identity used only to align comparable generic arms. */
function typeIdentity(type) {
  return type.getAliasSymbol?.()?.id ?? type.getSymbol?.()?.id ?? null;
}

/** Generic arguments retained by an alias or reference type. */
function typeArguments(checker, type) {
  const aliases = type.getAliasTypeArguments?.() ?? [];
  if (aliases.length > 0) return aliases;
  return type.isTypeReference?.() ? checker.getTypeArguments(type) : [];
}

/**
 * Descends through unions and matching generic aliases until it reaches the
 * smallest checker-proven incompatible pair. No language construct or type
 * name is special-cased here.
 */
function incompatibleLeaf(checker, found, expected, TypeFlags, depth = 0) {
  if (depth >= 8 || checker.isTypeAssignableTo(found, expected)) return null;

  const identity = typeIdentity(found);
  if (identity !== null) {
    const counterpart = typeConstituents(expected).find(
      (candidate) => typeIdentity(candidate) === identity,
    );
    if (counterpart) {
      const foundArgs = typeArguments(checker, found);
      const expectedArgs = typeArguments(checker, counterpart);
      if (foundArgs.length === expectedArgs.length && foundArgs.length > 0) {
        for (let i = 0; i < foundArgs.length; i++) {
          if (!checker.isTypeAssignableTo(foundArgs[i], expectedArgs[i])) {
            return (
              incompatibleLeaf(checker, foundArgs[i], expectedArgs[i], TypeFlags, depth + 1) ??
              relationPair(checker, foundArgs[i], expectedArgs[i], TypeFlags)
            );
          }
        }
      }
      // Two instantiations of one declaration with no retained type
      // arguments (an instantiated object literal, e.g. a lowered variant
      // case) differ where a declared property differs.
      const property = propertyLeaf(checker, found, counterpart, TypeFlags, depth);
      if (property) return property;
    }
  }
  // Two single-signature function types differ where their results (or a
  // parameter) differ — a pipeline `flow` boundary is the canonical case.
  // The signature API is optional on the native bridge; without it the
  // complete function types remain the leaf.
  try {
    const foundCalls = found.getCallSignatures?.() ?? [];
    const expectedCalls = expected.getCallSignatures?.() ?? [];
    if (foundCalls.length === 1 && expectedCalls.length === 1) {
      const foundReturn = foundCalls[0].getReturnType?.();
      const expectedReturn = expectedCalls[0].getReturnType?.();
      if (
        foundReturn &&
        expectedReturn &&
        !checker.isTypeAssignableTo(foundReturn, expectedReturn)
      ) {
        return (
          incompatibleLeaf(checker, foundReturn, expectedReturn, TypeFlags, depth + 1) ??
          relationPair(checker, foundReturn, expectedReturn, TypeFlags)
        );
      }
    }
  } catch {
    // Fall through to the complete pair.
  }
  return relationPair(checker, found, expected, TypeFlags);
}

/**
 * Where two instantiations of one declaration differ: the single declared
 * property whose types are incompatible, descended recursively. Reached
 * only through an identity-matched counterpart, so apparent members of
 * primitives never qualify. Anything ambiguous — no shared properties, or
 * more than one differing — keeps the complete pair. The property APIs are
 * optional on the native bridge.
 */
function propertyLeaf(checker, found, expected, TypeFlags, depth) {
  try {
    const properties = found.getProperties?.() ?? [];
    if (properties.length === 0) return null;
    let shared = 0;
    let incompatible = 0;
    let pair = null;
    for (const property of properties) {
      const name = property.getName?.() ?? property.name;
      if (!name) continue;
      const counterpart = checker.getPropertyOfType(expected, name);
      if (!counterpart) continue;
      shared += 1;
      const foundType = checker.getTypeOfSymbol(property);
      const expectedType = checker.getTypeOfSymbol(counterpart);
      if (!checker.isTypeAssignableTo(foundType, expectedType)) {
        incompatible += 1;
        pair = { foundType, expectedType };
      }
    }
    if (shared === 0 || incompatible !== 1 || !pair) return null;
    return (
      incompatibleLeaf(checker, pair.foundType, pair.expectedType, TypeFlags, depth + 1) ??
      relationPair(checker, pair.foundType, pair.expectedType, TypeFlags)
    );
  } catch {
    return null;
  }
}

function relationPair(checker, found, expected, TypeFlags) {
  const generalized =
    !(expected.flags & TypeFlags.Never) &&
    isLiteralType(found, TypeFlags) &&
    !couldHaveTopLevelSingletonTypes(checker, expected, TypeFlags);
  const shown = generalized ? checker.getBaseTypeOfLiteralType(found) : found;
  return { expected: checker.typeToString(expected), found: checker.typeToString(shown) };
}

function isLiteralType(type, TypeFlags) {
  if (type.flags & TypeFlags.Boolean) return true;
  if (type.flags & TypeFlags.Union) {
    return !!(type.flags & TypeFlags.EnumLiteral) ||
      typeConstituents(type).every((member) => !!(member.flags & TypeFlags.Unit));
  }
  return !!(type.flags & TypeFlags.Unit);
}

function couldHaveTopLevelSingletonTypes(checker, type, TypeFlags) {
  if (type.flags & TypeFlags.Boolean) return false;
  if (type.flags & TypeFlags.UnionOrIntersection) {
    return (type.getTypes?.() ?? []).some((member) => couldHaveTopLevelSingletonTypes(checker, member, TypeFlags));
  }
  if (type.flags & TypeFlags.Instantiable) {
    const constraint = type.flags & TypeFlags.TypeParameter
      ? checker.getConstraintOfTypeParameter(type)
      : checker.getBaseConstraintOfType(type);
    if (constraint && constraint !== type) return couldHaveTopLevelSingletonTypes(checker, constraint, TypeFlags);
  }
  return !!(type.flags & (TypeFlags.Unit | TypeFlags.TemplateLiteral | TypeFlags.StringMapping));
}

function incompatibleLeaves(checker, found, expected, TypeFlags) {
  const leaves = [];
  const seen = new Set();
  const constituents = typeConstituents(found);
  const wholeExpected = checker.typeToString(expected);
  let unreduced = constituents.length > 1;
  for (const constituent of constituents) {
    if (checker.isTypeAssignableTo(constituent, expected)) {
      unreduced = false;
      continue;
    }
    const leaf = incompatibleLeaf(checker, constituent, expected, TypeFlags);
    if (!leaf) {
      unreduced = false;
      continue;
    }
    if (leaf.expected !== wholeExpected || leaf.found !== relationPair(checker, constituent, expected, TypeFlags).found) {
      unreduced = false;
    }
    const key = `${leaf.expected}\0${leaf.found}`;
    if (seen.has(key)) continue;
    seen.add(key);
    leaves.push(leaf);
  }
  if (unreduced) return [relationPair(checker, found, expected, TypeFlags)];
  return leaves;
}

/**
 * Replaces the served modules with `modules`, reporting what changed so the
 * snapshot can be updated rather than rebuilt.
 */
function serve(files, dirs, modules) {
  const changed = [];
  const created = [];
  const deleted = [];
  const seen = new Set();
  for (const module of modules) {
    seen.add(module.path);
    if (!files.has(module.path)) {
      (fs.existsSync(module.path) ? changed : created).push(module.path);
    }
    else if (files.get(module.path) !== module.text) changed.push(module.path);
    files.set(module.path, module.text);
    for (let d = path.dirname(module.path); d && d !== path.dirname(d); d = path.dirname(d)) {
      dirs.add(d);
    }
  }
  for (const known of [...files.keys()]) {
    if (!seen.has(known)) {
      // Releasing an overlay reveals a real host file again. It is a text
      // change, not a deletion from the TypeScript project graph.
      (fs.existsSync(known) ? changed : deleted).push(known);
      files.delete(known);
    }
  }
  return { changed, created, deleted };
}

/**
 * The literals of `type` that `covered` does not, or `null` when the type is
 * not a definite finite union of literals. Anything less definite — `string`,
 * a type parameter, `"a" | string` — is left alone: a missed diagnostic
 * beats a false one.
 */
function missingLiterals(type, covered, constituentsOf) {
  if (!type) return null;
  const constituents = constituentsOf(type);
  const values = [];
  for (const c of constituents) {
    const value = literalValue(c);
    if (value === undefined) return null;
    values.push(value);
  }
  const seen = new Set(covered.map((c) => JSON.stringify(c)));
  const missing = values.filter((v) => !seen.has(JSON.stringify(v)));
  return missing.length > 0 ? missing : null;
}

/**
 * The `kind` property symbols of `type`'s constituents, or `null` when the
 * type is not a union of tagged object types — a bare object, a type
 * parameter or `any` makes the whole question indefinite, and an indefinite
 * question gets no answer. Whether each `kind` is a single string literal is
 * the caller's half, batched over every tag check at once.
 */
function tagKindSymbols(type, constituentsOf, kindSymbolOf) {
  if (!type) return null;
  const constituents = constituentsOf(type);
  const symbols = [];
  for (const c of constituents) {
    const kind = kindSymbolOf(c);
    if (!kind) return null;
    symbols.push(kind);
  }
  return symbols;
}

function isDefiniteResult(type, constituentsOf, kindSymbolOf, checker) {
  if (!type) return false;
  const constituents = constituentsOf(type);
  if (constituents.length !== 2) return false;
  const tags = new Set();
  for (const constituent of constituents) {
    const kind = kindSymbolOf(constituent);
    if (!kind) return false;
    const tag = literalValue(checker.getTypeOfSymbol(kind));
    if (tag !== "Ok" && tag !== "Err") return false;
    const payload = checker.getPropertyOfType(
      constituent,
      tag === "Ok" ? "value" : "error",
    );
    if (!payload) return false;
    tags.add(tag);
  }
  return tags.size === 2;
}

function declaredBinding(checker, symbol, check, { SyntaxKind, SymbolFlags }) {
  if (!symbol || !(symbol.flags & SymbolFlags.Property) ||
      symbol.valueDeclaration?.kind !== SyntaxKind.Parameter) {
    return symbol;
  }
  const parameter = checker.resolveName(
    symbol.name,
    SymbolFlags.FunctionScopedVariable,
    { document: check.module, position: check.start },
    true,
  );
  return parameter?.valueDeclaration?.kind === SyntaxKind.Parameter ? parameter : symbol;
}

/**
 * Runs `ask(module, positions)` once per module over `entries` — each
 * `{ check: { module, start } }` — and returns the answers aligned with
 * `entries`' own order. The grouping is invisible to the caller: entry `i`'s
 * answer is at `i`, whatever module it was grouped under.
 */
function perModule(entries, ask) {
  const byModule = new Map();
  entries.forEach((entry, at) => {
    let group = byModule.get(entry.check.module);
    if (!group) byModule.set(entry.check.module, (group = []));
    group.push(at);
  });
  const answers = new Array(entries.length);
  for (const [module, group] of byModule) {
    const batch = ask(module, group.map((at) => entries[at].check.start));
    group.forEach((at, i) => (answers[at] = batch[i]));
  }
  return answers;
}

/** `checker`, calling `called` once per request it sends to the compiler. */
function countedChecker(checker, called) {
  return new Proxy(checker, {
    get(target, key) {
      const value = target[key];
      if (typeof value !== "function") return value;
      return (...args) => {
        called();
        return value.apply(target, args);
      };
    },
  });
}

/**
 * `checker`, with the answers that depend only on their arguments kept for
 * the life of one ask: a node's symbol and type, and the relations and
 * conversions of types, keyed by type id (snapshot-scoped). A node list
 * asks for the nodes not yet answered in one batch. The answers are the
 * checker's own; only the number of round trips changes.
 */
function memoizedChecker(checker) {
  const memo = (cache, key, compute) => {
    if (!cache.has(key)) cache.set(key, compute());
    return cache.get(key);
  };
  const perNode = (name, method) => {
    const cache = new Map();
    return (arg) => {
      if (!Array.isArray(arg)) return memo(cache, arg, () => checker[method](arg));
      const missing = [...new Set(arg.filter((node) => !cache.has(node)))];
      if (missing.length > 0) {
        const answers = batched(
          name,
          () => checker[method](missing),
          () => missing.map((node) => checker[method](node)),
        );
        missing.forEach((node, index) => cache.set(node, answers[index]));
      }
      return arg.map((node) => cache.get(node));
    };
  };
  const assignable = new Map();
  const widened = new Map();
  const base = new Map();
  const own = {
    getSymbolAtLocation: perNode("symbolsAtLocation", "getSymbolAtLocation"),
    getTypeAtLocation: perNode("typesAtLocation", "getTypeAtLocation"),
    isTypeAssignableTo: (source, target) =>
      memo(assignable, `${source.id}:${target.id}`, () => checker.isTypeAssignableTo(source, target)),
    getWidenedType: (type) => memo(widened, type.id, () => checker.getWidenedType(type)),
    getBaseTypeOfLiteralType: (type) => memo(base, type.id, () => checker.getBaseTypeOfLiteralType(type)),
  };
  return new Proxy(checker, {
    get(target, key) {
      if (Object.hasOwn(own, key)) return own[key];
      const value = target[key];
      return typeof value === "function" ? value.bind(target) : value;
    },
  });
}

/**
 * Whether each batched checker endpoint is served by the resolved client,
 * discovered on first use. A released client older than the batch overloads
 * still answers — one position at a time.
 */
const batchable = {};

/**
 * `batch()` when the client supports it, `single()` otherwise. The two
 * compute the same answers; only the number of round trips differs, so a
 * client without the batch endpoint changes no verdict.
 */
function batched(name, batch, single) {
  if (batchable[name] !== false) {
    try {
      const result = batch();
      if (Array.isArray(result)) {
        batchable[name] = true;
        return result;
      }
    } catch {
      // fall through — an endpoint the server does not serve
    }
    batchable[name] = false;
  }
  return single();
}

/** The value of a literal type, or `undefined` when it is not one. */
function literalValue(type) {
  const v = type.value;
  if (typeof v === "string" || typeof v === "number" || typeof v === "boolean") return v;
  if (typeof v === "bigint") return { bigint: v.toString() };
  return undefined;
}

await main();
