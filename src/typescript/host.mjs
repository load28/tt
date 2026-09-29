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
 *   open   { apiModule, cwd, tsconfig (nullable) }
 *       →  { ok: true }
 *
 *   ask    { modules: [{ path, text }],   // lowered .tt → x.tt.ts / x.ttx.tsx
 *            roots: [path],               // requested and open modules
 *            literalChecks: [{ module, start, covered: [...] }],
 *            tagChecks: [{ module, start, covered: [...] }],
 *            symbolChecks: [{ module, start }],
 *            resultShapeChecks: [{ module, start, end }],
 *            emitDeclarations: boolean }
 *       →  { diagnostics: [{ file, start, end, code, message, mismatch? }],
 *            literalMissing: [{ index, missing }],
 *            tagMissing: [{ index, missing }],
 *            tagMembers: [{ index, tags }],
 *            symbols: [{ index, id, name, builtin }],
 *            resultShapes: [{ index }],
 *            declarations: [{ path, text }] }
 *
 * An `ask` may also answer `{ error: "..." }`, which fails that request
 * without ending the session. EOF on stdin ends it.
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
import process from "node:process";
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
const CANNOT_READ_FILE = 5083;
const LOWERED = /\.(?:tt\.ts|ttx\.tsx)$/;
const TT_SOURCE = /\.ttx?$/;
const MAPPED_DECLARATION = /\.d\.(ttx?)\.ts$/;
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
  const buffer = Buffer.from(text + "\n", "utf8");
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
        if (e.code === "EAGAIN") continue;
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

function layeredFileSystem(files, aliases, dirs, configFiles, dependencies, listings, links) {
  // The packages this host publishes and links into the project are its
  // own, not project inputs: they are neither dependencies nor listings.
  const published = (p) => [...links].some(([link, target]) =>
    [link, target].some((root) => p === root || p.startsWith(root + "/")));
  return {
    // A `.tt` source the engine did not serve does not exist for TypeScript:
    // its text is tt, not the lowered module.
    fileExists: (f) => (files.has(f) ? true : TT_SOURCE.test(f) ? false : undefined),
    // `undefined` falls back to the real disk; `null` would mean "absent".
    readFile: (f) => {
      if (!files.has(f) && !dependencies.has(f) && !published(f)) dependencies.set(f, diskVersion(f));
      if (configFiles.has(f)) return configFiles.get(f);
      if (files.has(f)) return files.get(f);
      return TT_SOURCE.test(f) ? null : undefined;
    },
    directoryExists: (d) => (dirs.has(d) ? true : undefined),
    realpath: (p) => {
      for (const [link, target] of links) {
        if (p === link || p.startsWith(link + "/")) return target + p.slice(link.length);
      }
      return undefined;
    },
    getAccessibleEntries: (d) => {
      let real = { files: [], directories: [] };
      try {
        for (const e of fs.readdirSync(d, { withFileTypes: true })) {
          if (e.isDirectory()) real.directories.push(e.name);
          else real.files.push(e.name);
        }
      } catch {
        if (!dirs.has(d)) return undefined;
      }
      if (!published(d)) listings.set(d, new Set([...real.files, ...real.directories]));
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
  let isExpression;
  let isIdentifier;
  let isVariableDeclaration;
  let isBinaryExpression;
  let isTypeReferenceNode;
  let isTypeQueryNode;
  let isQualifiedName;
  let SyntaxKind;
  try {
    ({ API, SymbolFlags, TypeFlags, NodeBuilderFlags } = await import(open.apiModule));
    ({
      isExpression,
      isIdentifier,
      isVariableDeclaration,
      isBinaryExpression,
      isTypeReferenceNode,
      isTypeQueryNode,
      isQualifiedName,
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
  const dependencies = new Map();
  const listings = new Map();
  const links = new Map();
  const aliases = new Set();
  const pendingDisk = { created: [], changed: [], deleted: [] };
  let diskGeneration = 0;
  let mapped = false;
  const mapperPackage = path.join(
    path.dirname(fileURLToPath(import.meta.url)),
    `typed-engine-mapper-${createHash("sha256").update(process.execPath).digest("hex").slice(0, 16)}`,
  );
  // The client runs the executable shipped beside it — the one it was
  // built against, and the same one ttc drives as a language server.
  const connect = () => new API({
    cwd: open.cwd,
    runExternalCode: mapped,
    fs: layeredFileSystem(files, aliases, dirs, configFiles, dependencies, listings, links),
  });
  let api = connect();
  writeLine(JSON.stringify({ ok: true }));

  let opened = false;
  const openRoots = new Set();
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
          answer = handle(job);
        }
      } catch (e) {
        // The Rust boundary classifies this as an internal compiler error.
        // Keep the protocol response actionable without leaking a Node
        // implementation stack into the user's diagnostic stream.
        answer = { error: e instanceof Error ? e.message : String(e) };
      }
      writeLine(JSON.stringify(answer));
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
      literalChecks: (job.literalChecks ?? []).map(module),
      tagChecks: (job.tagChecks ?? []).map(module),
      symbolChecks: (job.symbolChecks ?? []).map(module),
      resultShapeChecks: (job.resultShapeChecks ?? []).map(module),
      contextualSlots: (job.contextualSlots ?? []).map(module),
    };
  }

  function engineAnswer(out) {
    for (const diagnostic of out.diagnostics) {
      diagnostic.file = moduleName(diagnostic.file);
      for (const related of diagnostic.related ?? []) related.file = moduleName(related.file);
      if (diagnostic.mismatch?.declaration) {
        diagnostic.mismatch.declaration.file = moduleName(diagnostic.mismatch.declaration.file);
      }
    }
    out.projectModules = out.projectModules.map(moduleName);
    for (const declaration of out.declarations) {
      if (mapped) declaration.path = declaration.path.replace(MAPPED_DECLARATION, ".$1.d.ts");
    }
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
    api.close();
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
      projectDiagnostics: [],
      literalMissing: [],
      tagMissing: [],
      tagMembers: [],
      symbols: [],
      resultShapes: [],
      declarations: [],
      contextualSlots: [],
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
      // A configuration TypeScript cannot read is TS5083, the diagnostic
      // `tsc` reports for it. No project exists until it can be read again,
      // and then it is opened afresh.
      const unreadable = api.readConfigFile(open.tsconfig).error;
      if (unreadable?.code === CANNOT_READ_FILE) {
        if (opened) reconnect();
        out.projectDiagnostics.push({ file: open.tsconfig, code: unreadable.code, message: unreadable.text });
        out.dependencies = [...dependencies.keys(), ...listings.keys()];
        return engineAnswer(out);
      }
      const wanted = !foreignMappers(api.parseConfigFile(open.tsconfig));
      if (wanted !== mapped) {
        mapped = wanted;
        reconnect();
        changes = serve(files, dirs, servedModules(job.modules ?? []));
        api.parseConfigFile(open.tsconfig);
      }
      for (const file of new Set([open.tsconfig, ...dependencies.keys()])) {
        if (!file.endsWith(".json") || !(files.has(file) || fs.existsSync(file))) continue;
        const { config, error } = api.readConfigFile(file);
        if (error || !config || typeof config !== "object") continue;
        let changed = false;
        // Unmapped, a pattern naming `.tt` names the lowered `.tt.ts`. Mapped,
        // user patterns already name what TypeScript sees; only a
        // configuration the engine serves names its modules by the engine's
        // `x.tt.ts`.
        const rename = mapped
          ? (files.has(file) ? served : null)
          : (entry) => entry + (entry.endsWith(".ttx") ? ".tsx" : entry.endsWith(".tt") ? ".ts" : "");
        for (const key of rename ? ["files", "include", "exclude"] : []) {
          if (!Array.isArray(config[key])) continue;
          config[key] = config[key].map(entry => {
            if (typeof entry !== "string") return entry;
            const renamed = rename(entry);
            changed ||= renamed !== entry;
            return renamed;
          });
        }
        if (Array.isArray(config.contentMappers)) {
          const mappers = config.contentMappers
            .map(entry => {
              if (!entry || typeof entry !== "object" || !Array.isArray(entry.extensions)) return entry;
              const extensions = entry.extensions.filter(extension => extension !== ".tt" && extension !== ".ttx");
              if (extensions.length === entry.extensions.length) return entry;
              changed = true;
              return extensions.length > 0 ? { ...entry, extensions } : null;
            })
            .filter(entry => entry !== null);
          if (mappers.length > 0) config.contentMappers = mappers;
          else delete config.contentMappers;
        }
        if (mapped && path.resolve(file) === path.resolve(open.tsconfig)) {
          config.contentMappers = [{ package: MAPPER_PACKAGE, extensions: [".tt", ".ttx"] }];
          changed = true;
        }
        if (changed) configFiles.set(file, JSON.stringify(config));
      }
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
    let snapshot = api.updateSnapshot(params);
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
      snapshot = api.updateSnapshot({ openFiles: opening, closeFiles: closing });
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

    const contextual = ({ project, members }) => {
      const checker = project.checker;
      // The storage the lowering declared in each module, annotated or not.
      // TypeScript can name a type after it (a class expression assigned
      // to it is `typeof $tt_v0`), and an annotation that did would read
      // the compiler's glue, or itself (TS2502).
      const storage = new Map();
      const storageOf = (module, source) => {
        let symbols = storage.get(module);
        if (symbols) return symbols;
        const ends = new Set((job.contextualSlots ?? [])
          .filter((slot) => slot.module === module)
          .map((slot) => slot.declarationEnd));
        symbols = new Set();
        const collect = (node) => {
          if (isVariableDeclaration(node) && isIdentifier(node.name) && ends.has(node.name.end)) {
            const declared = checker.getSymbolAtLocation(node.name);
            if (declared) symbols.add(declared.id);
          }
          node.forEachChild(collect);
        };
        collect(source);
        storage.set(module, symbols);
        return symbols;
      };
      for (const [index, slot] of (job.contextualSlots ?? []).entries()) {
        if (slot.annotated || !members.has(slot.module)) continue;
        const source = project.program.getSourceFile(slot.module);
        if (!source) continue;
        let declaration;
        const identifiers = [];
        const assignments = [];
        const visit = (node) => {
          if (isVariableDeclaration(node) && isIdentifier(node.name) &&
              node.name.end === slot.declarationEnd && !node.type) declaration = node;
          if (isIdentifier(node)) identifiers.push(node);
          if (isBinaryExpression(node) && node.operatorToken.kind === SyntaxKind.EqualsToken && isIdentifier(node.left)) assignments.push(node);
          node.forEachChild(visit);
        };
        visit(source);
        if (!declaration) continue;
        const symbol = checker.getSymbolAtLocation(declaration.name);
        if (!symbol) continue;
        const declaredType = declaration.initializer ? checker.getTypeAtLocation(declaration.name) : undefined;
        // The annotation is written at the storage declaration, which may
        // enclose the scope the type was observed in: a class declared in a
        // match arm's block is out of scope there, or an outer declaration
        // of the same name shadows it. An annotation is written only when
        // every name it references denotes, at the declaration, the symbol
        // it denotes where the type was observed, and that symbol is not
        // storage the lowering declared. It is written without truncation:
        // a truncated type is not the type (`... 3 more ...` is not even
        // TypeScript).
        const generated = storageOf(slot.module, source);
        const annotation = (type, observed) => {
          const node = typeNode(checker, type, declaration, NodeBuilderFlags.NoTruncation);
          if (!node) return undefined;
          let accessible = true;
          const visit = (child) => {
            if (!accessible) return;
            const reference = isTypeReferenceNode(child)
              ? [child.typeName, SymbolFlags.Type]
              : isTypeQueryNode(child) ? [child.exprName, SymbolFlags.Value] : undefined;
            if (reference) {
              let [name, meaning] = reference;
              while (isQualifiedName(name)) {
                name = name.left;
                meaning = SymbolFlags.Namespace | SymbolFlags.Value;
              }
              const here = checker.resolveName(name.text, meaning, declaration);
              const there = checker.resolveName(name.text, meaning, observed);
              accessible = !!here && !!there && here.id === there.id && !generated.has(here.id);
            }
            child.forEachChild(visit);
          };
          visit(node);
          return accessible ? project.emitter.printNode(node) : undefined;
        };
        let expected;
        let observedAt;
        let ambiguous = false;
        for (const identifier of identifiers) {
          if (identifier === declaration.name || identifier.text !== declaration.name.text) continue;
          if (checker.getSymbolAtLocation(identifier)?.id !== symbol.id) continue;
          // A use narrowed by control flow cannot supply the declaration's
          // type: doing so would reject the initializer's other constituents.
          if (declaredType && checker.getTypeAtLocation(identifier).id !== declaredType.id) continue;
          const context = checker.getContextualType(identifier);
          if (!context || (context.flags & (TypeFlags.Any | TypeFlags.Unknown)) || context.isErrorType()) continue;
          if (expected && expected.id !== context.id) { ambiguous = true; break; }
          expected = context;
          observedAt ??= identifier;
        }
        if (job.inferJoinTypes && !expected && !ambiguous && !declaration.initializer) {
          // A statement join must have the union of its incoming value types.
          // In particular, TS's evolving-array inference at assignment sites is
          // not expression inference. Ask for each RHS type in its branch scope
          // and serialize it at the declaration; never infer from diagnostic text.
          const incoming = assignments.filter(assignment =>
            assignment.left.text === declaration.name.text &&
            checker.getSymbolAtLocation(assignment.left)?.id === symbol.id);
          const types = incoming.map(assignment =>
            checker.getWidenedType(checker.getBaseTypeOfLiteralType(checker.getTypeAtLocation(assignment.right))));
          if (!types.length || types.some(type => (type.flags & (TypeFlags.Any | TypeFlags.Unknown)) || type.isErrorType())) continue;
          // Remove constituents subsumed by another incoming type. This is the
          // checker's assignability relation, including never[] <: number[].
          const joined = types.flatMap((type, index) => types.some((other, otherIndex) =>
            index !== otherIndex && checker.isTypeAssignableTo(type, other) &&
            (!checker.isTypeAssignableTo(other, type) || otherIndex < index)) ? [] : [index]);
          const annotations = joined.map(index => annotation(types[index], incoming[index].right));
          if (annotations.length && annotations.every(Boolean)) {
            out.contextualSlots.push({ index, annotation: annotations.length === 1
              ? annotations[0] : annotations.map(t => `(${t})`).join(" | ") });
          }
          continue;
        }
        if (!expected || ambiguous) continue;
        const text = annotation(expected, observedAt);
        if (text) out.contextualSlots.push({ index, annotation: text });
      }
    };
    for (const group of groups) contextual(group);
    if (job.contextualOnly) { out.dependencies = [...dependencies.keys(), ...listings.keys()]; return engineAnswer(out); }
    const reported = new Set();
    const unique = (diagnostics) => diagnostics.filter((d) => {
      const key = JSON.stringify([d.fileName ?? null, d.pos, d.end, d.code, d.text]);
      if (reported.has(key)) return false;
      reported.add(key);
      return true;
    });
    // The whole configured program, not just the lowered modules: a
    // hand-written `.ts` and an `.tt` are in one project, so an error in
    // either is this run's to report. Which file it lands in decides how it
    // is positioned, and that is ttc's half. Another project answers only
    // for the requested modules it is the default project of.
    const answer = ({ project, members, whole }) => {
      const checker = project.checker;
      const program = project.program;
      const scope = whole ? undefined : [...members];
      const structural = unique([
        ...(whole
          ? [
            ...program.getConfigFileParsingDiagnostics(),
            ...program.getProgramDiagnostics(),
            ...program.getGlobalDiagnostics(),
          ]
          : []),
        ...program.getSyntacticDiagnostics(scope),
      ]);
      const semantic = unique(program.getSemanticDiagnostics(scope));
      const late = whole ? unique(program.getGlobalDiagnostics()) : [];
      for (const d of [...structural, ...late]) {
        if (!d.fileName || configFiles.has(d.fileName) || d.pos < 0) {
          if (open.tsconfig && whole) out.projectDiagnostics.push({ file: d.fileName ?? null, code: d.code, message: d.text });
          continue;
        }
        out.diagnostics.push({ file: d.fileName, start: d.pos, end: d.end, code: d.code, message: d.text });
      }
      for (const d of semantic) {
        if (!d.fileName) {
          if (open.tsconfig && whole) out.projectDiagnostics.push({ file: null, code: d.code, message: d.text });
          continue;
        }
        const mismatch = contextualMismatch(project, checker, d, isExpression);
        const related = relatedPlaces(d);
        out.diagnostics.push({
          file: d.fileName,
          start: d.pos,
          end: d.end,
          code: d.code,
          message: d.text,
          ...(mismatch ? { mismatch } : {}),
          ...(related.length > 0 ? { related } : {}),
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
        const symbol = symbols[at];
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
    for (const group of groups) answer(group);
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
      const emitted = project.program.getDeclarationEmit(
        (job.modules ?? []).map((m) => served(m.path)).filter((module) => projectModules.has(module)),
      );
      for (const [path, file] of emitted.outputFiles) {
        out.declarations.push({ path, text: file.text });
      }
    }
    out.dependencies = [...dependencies.keys(), ...listings.keys()];
    return engineAnswer(out);
  }
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
function typeNode(checker, type, location, flags) {
  try {
    return checker.typeToTypeNode(type, location, flags);
  } catch (error) {
    checker.typeToString(type, location);
    return undefined;
  }
}

/**
 * Finds the expression TypeScript compared with a contextual type for a
 * diagnostic. This is syntax-neutral: return values, annotated initializers,
 * call arguments and future lowered constructs all participate through the
 * checker’s contextual typing relation.
 */
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

function contextualMismatch(project, checker, diagnostic, isExpression) {
  const sourceFile = project.program.getSourceFile(diagnostic.fileName);
  if (!sourceFile) return null;

  const chain = [];
  const visit = (node) => {
    if (node.pos > diagnostic.pos || node.end < diagnostic.end) return;
    chain.push(node);
    node.forEachChild(visit);
  };
  visit(sourceFile);

  for (let i = chain.length - 1; i >= 0; i--) {
    const node = chain[i];
    const candidates = [];
    if (isExpression(node)) candidates.push(node);
    node.forEachChild((child) => {
      if (isExpression(child)) candidates.push(child);
    });
    candidates.sort((left, right) => right.getWidth(sourceFile) - left.getWidth(sourceFile));
    for (const expression of candidates) {
      let found;
      let expected;
      try {
        found = checker.getTypeAtLocation(expression);
        expected = checker.getContextualType(expression);
      } catch {
        continue;
      }
      if (!found || !expected || found.isErrorType?.() || expected.isErrorType?.()) continue;
      if (checker.isTypeAssignableTo(found, expected)) continue;
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
        differences: incompatibleLeaves(checker, found, expected),
        ...(declaration ? { declaration } : {}),
      };
    }
  }
  return null;
}

/** The innermost expression whose source range contains the emitted value. */
function smallestExpressionCovering(sourceFile, start, end, isExpression) {
  let found = null;
  const visit = (node) => {
    if (node.getStart(sourceFile) > start || node.end < end) return;
    if (isExpression(node)) found = node;
    node.forEachChild(visit);
  };
  visit(sourceFile);
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
function incompatibleLeaf(checker, found, expected, depth = 0) {
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
              incompatibleLeaf(checker, foundArgs[i], expectedArgs[i], depth + 1) ?? {
                expected: checker.typeToString(expectedArgs[i]),
                found: checker.typeToString(foundArgs[i]),
              }
            );
          }
        }
      }
      // Two instantiations of one declaration with no retained type
      // arguments (an instantiated object literal, e.g. a lowered variant
      // case) differ where a declared property differs.
      const property = propertyLeaf(checker, found, counterpart, depth);
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
          incompatibleLeaf(checker, foundReturn, expectedReturn, depth + 1) ?? {
            expected: checker.typeToString(expectedReturn),
            found: checker.typeToString(foundReturn),
          }
        );
      }
    }
  } catch {
    // Fall through to the complete pair.
  }
  return {
    expected: checker.typeToString(expected),
    found: checker.typeToString(found),
  };
}

/**
 * Where two instantiations of one declaration differ: the single declared
 * property whose types are incompatible, descended recursively. Reached
 * only through an identity-matched counterpart, so apparent members of
 * primitives never qualify. Anything ambiguous — no shared properties, or
 * more than one differing — keeps the complete pair. The property APIs are
 * optional on the native bridge.
 */
function propertyLeaf(checker, found, expected, depth) {
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
      incompatibleLeaf(checker, pair.foundType, pair.expectedType, depth + 1) ?? {
        expected: checker.typeToString(pair.expectedType),
        found: checker.typeToString(pair.foundType),
      }
    );
  } catch {
    return null;
  }
}

function incompatibleLeaves(checker, found, expected) {
  const leaves = [];
  const seen = new Set();
  for (const constituent of typeConstituents(found)) {
    if (checker.isTypeAssignableTo(constituent, expected)) continue;
    const leaf = incompatibleLeaf(checker, constituent, expected);
    if (!leaf) continue;
    const key = `${leaf.expected}\0${leaf.found}`;
    if (seen.has(key)) continue;
    seen.add(key);
    leaves.push(leaf);
  }
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
