import * as fs from "node:fs";
import * as path from "node:path";
import * as readline from "node:readline";

const { API } = await import(process.argv[2]);

const UNKNOWN_OPTION = 5023;
const UNKNOWN_OPTION_SUGGESTED = 5025;
const CONFIG_FILE_ONLY = 6064;
const MAX_CONFIGURATIONS = 25;
const RELATIVE_SUFFIXES = [".ts", ".tsx", ".d.ts", "/index.ts", "/index.tsx", "/index.d.ts"];

let api = new API({ cwd: "/" });

function messageText(diagnostic, level = 0) {
  let text = (level > 0 ? "\n" + "  ".repeat(level) : "") + diagnostic.text;
  for (const child of diagnostic.messageChain ?? []) text += messageText(child, level + 1);
  return text;
}

function relative(value) {
  return value.startsWith("/") ? "." + value : value;
}

function variations(value) {
  const entries = value.split(",").map((entry) => entry.trim()).filter(Boolean);
  if (entries.some((entry) => entry === "*" || entry.startsWith("!") || entry.startsWith("-"))) return null;
  return entries;
}

function typedValue(name, value) {
  const parsed = api.parseCommandLine(["--" + name, value]);
  const codes = (parsed.errors ?? []).map((error) => error.code);
  if (codes.includes(UNKNOWN_OPTION) || codes.includes(UNKNOWN_OPTION_SUGGESTED)) return { unknown: true };
  if (codes.includes(CONFIG_FILE_ONLY)) {
    try {
      return { key: name, value: JSON.parse(value) };
    } catch (error) {
      return { invalid: error.message };
    }
  }
  if (codes.length > 0) return { invalid: parsed.errors.map((error) => error.text).join(" ") };
  const keys = Object.keys(parsed.options ?? {});
  if (keys.length !== 1) return { unknown: true };
  const key = keys[0];
  const typed = parsed.options[key];
  if (typeof typed === "boolean") return { key, value: typed };
  if (Array.isArray(typed)) {
    const items = value.split(",").map((item) => item.trim()).filter(Boolean);
    const paths = typed.every((item) => typeof item === "string" && item.startsWith("/"));
    return { key, value: paths ? items.map(relative) : items, list: true };
  }
  if (typeof typed === "number") {
    return { key, value: /^-?\d+$/.test(value.trim()) ? Number(value) : value.trim() };
  }
  if (typeof typed === "string" && typed.startsWith("/")) return { key, value: relative(value.trim()) };
  return { key, value: typeof typed === "string" ? value.trim() : typed };
}

function options(entries) {
  const unknown = [];
  const invalid = [];
  const varies = [];
  let configurations = [{}];
  for (const [name, raw] of entries) {
    const whole = typedValue(name, raw);
    if (whole.unknown) {
      unknown.push(name);
      continue;
    }
    let values;
    if (!whole.invalid && (whole.list || !/[,*]/.test(raw))) {
      values = [whole];
    } else {
      const each = variations(raw);
      if (!each) {
        varies.push([name, raw]);
        continue;
      }
      values = each.map((value) => typedValue(name, value));
    }
    const bad = values.find((value) => value.invalid || value.unknown);
    if (bad) {
      invalid.push([name, bad.invalid ?? "unknown"]);
      continue;
    }
    const seen = new Set();
    values = values.filter((value) => {
      const key = JSON.stringify(value.value);
      if (seen.has(key)) return false;
      seen.add(key);
      return true;
    });
    if (configurations.length * values.length > MAX_CONFIGURATIONS) {
      varies.push([name, raw]);
      continue;
    }
    configurations = configurations.flatMap((configuration) =>
      values.map((value) => ({ ...configuration, [value.key]: value.value }))
    );
  }
  return { configurations, unknown, invalid, varies };
}

function check(tsconfig, units) {
  const snapshot = api.updateSnapshot({ openProjects: [tsconfig] });
  try {
    const project = snapshot.getProject(tsconfig);
    if (!project) throw new Error("no project for " + tsconfig);
    const program = project.program;
    const compilerOptions = program.getCompilerOptions();
    const all = [...program.getConfigFileParsingDiagnostics()];
    const configured = all.length;
    const syntactic = new Set(program.getSyntacticDiagnostics());
    all.push(...syntactic);
    if (all.length === configured) {
      all.push(...program.getProgramDiagnostics());
      if (compilerOptions.listFilesOnly !== true) {
        if (all.length === configured) {
          all.push(...program.getSemanticDiagnostics());
          all.push(...program.getGlobalDiagnostics());
        }
        const declarations = compilerOptions.declaration === true || compilerOptions.composite === true;
        if (compilerOptions.noEmit === true && declarations && all.length === configured) {
          all.push(...program.getDeclarationDiagnostics());
        }
      }
    }
    const seen = new Set();
    const diagnostics = [];
    for (const d of all) {
      const entry = {
        file: d.fileName ?? null,
        start: d.fileName ? d.startPosition ?? null : null,
        end: d.fileName ? d.endPosition ?? null : null,
        code: d.code,
        category: d.category,
        message: messageText(d),
        syntactic: syntactic.has(d),
      };
      const key = JSON.stringify(entry);
      if (seen.has(key)) continue;
      seen.add(key);
      diagnostics.push(entry);
    }
    const members = new Set(units);
    const reached = new Set();
    for (const unit of units) {
      const file = program.getSourceFile(unit);
      if (!file) continue;
      for (const reference of file.referencedFiles ?? []) {
        const target = path.resolve(path.dirname(unit), reference.fileName);
        for (const candidate of [target, target + ".ts", target + ".tsx", target + ".d.ts"]) {
          if (members.has(candidate)) reached.add(candidate);
        }
      }
      const specifiers = [...(file.imports ?? []), ...(file.moduleAugmentations ?? [])];
      if (specifiers.length === 0) continue;
      const symbols = project.checker.getSymbolAtLocation(specifiers);
      symbols.forEach((symbol, index) => {
        const declarations = symbol?.declarations ?? [];
        for (const declaration of declarations) {
          const target = declaration.path ? String(declaration.path) : null;
          if (target && members.has(target)) reached.add(target);
        }
        const text = specifiers[index]?.text;
        if (declarations.length > 0 || typeof text !== "string" || !text.startsWith(".")) return;
        const target = path.resolve(path.dirname(unit), text);
        const base = target.replace(/\.(js|jsx|mjs|cjs)$/, "");
        for (const candidate of [target, ...RELATIVE_SUFFIXES.map((suffix) => base + suffix)]) {
          if (members.has(candidate)) reached.add(candidate);
        }
      });
    }
    for (const unit of units) {
      for (const source of declarationMapSources(unit)) {
        if (members.has(source)) reached.add(source);
      }
    }
    return { diagnostics, reached: [...reached] };
  } finally {
    api.updateSnapshot({ closeProjects: [tsconfig] });
  }
}

function readText(file) {
  try {
    return fs.readFileSync(file, "utf8");
  } catch {
    return null;
  }
}

function declarationMapSources(file) {
  if (!/\.d\.[cm]?ts$/.test(file)) return [];
  const text = readText(file);
  if (text === null) return [];
  let url = "";
  for (const raw of text.split(/\r\n|\r|\n|\u2028|\u2029/).reverse()) {
    const line = raw.trimStart();
    if (line.length === 0) continue;
    if (line.length < 4 || !line.startsWith("//") || (line[2] !== "#" && line[2] !== "@") || line[3] !== " ") break;
    if (line.startsWith("sourceMappingURL=", 4)) {
      url = line.slice(4 + "sourceMappingURL=".length).trimEnd();
      break;
    }
  }
  const candidates = [];
  if (url.startsWith("data:")) {
    const inline = /^data:application\/json;(?:charset=utf-8;)?base64,([A-Za-z0-9+/=]+)$/i.exec(url);
    if (inline) return mapSources(Buffer.from(inline[1], "base64").toString("utf8"), file);
  } else if (url) {
    candidates.push(url);
  }
  candidates.push(file + ".map");
  for (const candidate of candidates) {
    const map = path.resolve(path.dirname(file), candidate);
    const contents = readText(map);
    if (contents !== null) return mapSources(contents, map);
  }
  return [];
}

function mapSources(contents, map) {
  let parsed;
  try {
    parsed = JSON.parse(contents);
  } catch {
    return [];
  }
  const sources = Array.isArray(parsed?.sources) ? parsed.sources : [];
  if (sources.length === 0 || !parsed.file || !parsed.mappings) return [];
  if ((parsed.sourcesContent ?? []).some((content) => content !== null)) return [];
  const root = parsed.sourceRoot ? path.resolve(path.dirname(map), parsed.sourceRoot) : path.dirname(map);
  return sources.filter((source) => typeof source === "string").map((source) => path.resolve(root, source));
}

function programFiles(tsconfig) {
  const snapshot = api.updateSnapshot({ openProjects: [tsconfig] });
  try {
    const project = snapshot.getProject(tsconfig);
    if (!project) throw new Error("no project for " + tsconfig);
    return { files: [...project.program.getSourceFileNames()] };
  } finally {
    api.updateSnapshot({ closeProjects: [tsconfig] });
  }
}

const lines = readline.createInterface({ input: process.stdin });
for await (const line of lines) {
  if (!line.trim()) continue;
  let answer;
  try {
    const request = JSON.parse(line);
    answer = request.options
      ? options(request.options)
      : request.files
        ? programFiles(request.files)
        : check(request.check, request.units ?? []);
  } catch (error) {
    answer = { error: String(error?.stack ?? error) };
  }
  process.stdout.write(JSON.stringify(answer) + "\n");
}
api.close();
