/* --------------------------------------------------------------------------
 * @openload28/unplugin-tt — `.tt` modules for every bundler unplugin supports.
 *
 * The plugin resolves `.tt` specifiers itself and compiles each file with
 * `ttc` on the way in, so a project needs no intermediate `.ts` tree: the
 * bundler reads the sources directly.
 *
 * Two deliberate details:
 *
 * - `ttc` runs with `--rewrite-imports off`. Rewriting exists for the
 *   ahead-of-time pipeline (where a `.tt` neighbour has already become a
 *   `.ts` file); here the specifier must stay `.tt` so this plugin resolves
 *   it too.
 * - Module ids are the real file plus a query ending in `lang.ts` (or
 *   `lang.tsx`). ttc emits TypeScript, and the host's own TypeScript pass
 *   keys off that ending — this keeps the plugin out of that job entirely,
 *   while hosts that strip the query still find the file on disk. esbuild is told the loader explicitly instead, since
 *   its `load` hook may only return JavaScript.
 *
 * Editor support is separate: `ttc --types` writes the declarations that let
 * a `.ts` file import `.tt` without the type checker complaining.
 * ----------------------------------------------------------------------- */
import { execFile } from "node:child_process";
import * as fs from "node:fs";
import { createRequire } from "node:module";
import * as path from "node:path";
import { promisify } from "node:util";

import { createUnplugin } from "unplugin";

const run = promisify(execFile);

/**
 * Default compiler: the prebuilt binary from an installed `@openload28/tt-lang` npm
 * package when present (spawned directly — no per-call node launcher),
 * otherwise `ttc` from PATH as before.
 */
function defaultCompiler() {
  try {
    const require = createRequire(import.meta.url);
    return require("@openload28/tt-lang").binaryPath();
  } catch {
    return "ttc";
  }
}

const TS_SUFFIX = ".ts";
const TSX_SUFFIX = ".tsx";

const TT_FILE = /\.ttx?$/;
const cleanUrl = (id) => id.replace(/[?#][\s\S]*$/, "");
const SPECIAL_QUERY = /[?&](?:worker|sharedworker|raw|url)\b/;
const MODULE_MARKERS = new Set([`lang${TS_SUFFIX}`, `lang${TSX_SUFFIX}`]);
const moduleMarker = (file) => (file.endsWith(".ttx") ? `lang${TSX_SUFFIX}` : `lang${TS_SUFFIX}`);

const SCANNED_FILE = /\.ttx?(?:\?[^/]*)?$/;

const queryOf = (id, file) => id.slice(file.length).replace(/#[\s\S]*$/, "");

const moduleId = (file, query) => {
  const params = query.slice(1).split("&").filter((param) => param !== "" && !MODULE_MARKERS.has(param));
  return `${file}?${[...params, moduleMarker(file)].join("&")}`;
};

const sourceFileOfId = (id) => {
  const file = cleanUrl(id);
  if (!TT_FILE.test(file)) return null;
  const params = queryOf(id, file).slice(1).split("&");
  return params[params.length - 1] === moduleMarker(file) ? file : null;
};

/** The bare specifier tt sources use for the standard library. */
const STD_MODULES = new Map([
  ["@tt/std", "types"],
  ["@tt/std/option", "option"],
  ["@tt/std/result", "result"],
  ["@tt/runtime", "runtime"],
]);

const STD_ID_PREFIX = "virtual:unplugin-tt/std/";

/** Virtual module id for the standard library. */
const stdId = (module) => `${STD_ID_PREFIX}${module}${TS_SUFFIX}`;

const stdModuleOfId = (id) => {
  for (const module of STD_MODULES.values()) {
    if (id === stdId(module)) return module;
  }
  return null;
};

const nativePath = (file) => path.resolve(file);

const INLINE_MAP =
  /(\r?\n)\/\/# sourceMappingURL=data:application\/json;charset=utf-8;base64,([A-Za-z0-9+/=]+)(?:\r?\n)?$/;

/**
 * Splits ttc's inline source map back out of the printed output.
 *
 * The map travels inline because stdout carries one stream; a bundler wants
 * it as a separate object so it can compose it with everything downstream.
 * Output without a map is returned unchanged.
 *
 * @param {string} code
 * @returns {{ code: string, map: object | null }}
 */
function detachInlineSourceMap(code, file) {
  const found = INLINE_MAP.exec(code);
  if (found === null) return { code, map: null };
  try {
    const { sourceRoot, ...map } = JSON.parse(Buffer.from(found[2], "base64").toString("utf8"));
    const base = path.resolve(path.dirname(file), sourceRoot ?? "");
    map.sources = (map.sources ?? []).map((source) => (source === null ? null : path.resolve(base, source)));
    return { code: code.slice(0, found.index + found[1].length), map };
  } catch {
    // An unreadable map is not a reason to fail the build; the code is
    // still exactly what ttc produced.
    return { code, map: null };
  }
}

/**
 * @typedef {object} Options
 * @property {string} [compiler] Path to the ttc binary (default: the
 *   installed `@openload28/tt-lang` package's binary, falling back to `"ttc"` on PATH).
 * @property {boolean} [verify] Run ttc's output self-check (default: true).
 * @property {boolean} [sourcemap] Ask ttc for a source map and hand it to the
 *   host, so a stack trace and a debugger point at the `.tt` (default: true).
 */

/** @type {import("unplugin").UnpluginFactory<Options | undefined>} */
export const unpluginFactory = (options = {}) => {
  const compiler = options.compiler ?? defaultCompiler();
  const verify = options.verify ?? true;
  const sourcemap = options.sourcemap ?? true;
  const dependenciesByModule = new Map();
  let devServer;

  const printArgs = (file, withMap) => {
    const args = ["-p", "--rewrite-imports", "off"];
    if (!verify) args.push("--no-verify");
    // ttc prints the map into the output as a data: URL — the one form
    // that survives a pipe. It is split back out here so the host gets a
    // real map object and composes it with its own transforms.
    if (withMap) args.push("--source-map", "inline");
    args.push(file);
    return args;
  };

  const scanSource = async (id) => {
    const file = cleanUrl(id);
    const { stdout } = await run(compiler, printArgs(file, false), { maxBuffer: 16 * 1024 * 1024 });
    return { code: stdout, lang: file.endsWith(".ttx") ? "tsx" : "ts" };
  };

  const esbuildScanPlugin = {
    name: "@openload28/unplugin-tt:dep-scan",
    setup(build) {
      build.onLoad({ filter: SCANNED_FILE }, async (args) => {
        const { code, lang } = await scanSource(args.path);
        return { contents: code, loader: lang, resolveDir: path.dirname(cleanUrl(args.path)) };
      });
    },
  };

  const rolldownScanPlugin = {
    name: "@openload28/unplugin-tt:dep-scan",
    async load(id) {
      if (!SCANNED_FILE.test(id)) return null;
      const { code, lang } = await scanSource(id);
      return { code, moduleType: lang };
    },
  };

  return {
    name: "@openload28/unplugin-tt",
    // Ahead of the host's own resolution: `.tt` is not an extension it
    // knows. Rollup and esbuild ignore `enforce`, where plugin order is the
    // author's responsibility instead.
    enforce: "pre",

    resolveId(source, importer) {
      // The standard library has no file: ttc prints it on demand, so it
      // becomes a virtual module. Nothing lands in the project tree.
      const stdModule = STD_MODULES.get(source);
      if (stdModule !== undefined) return stdId(stdModule);
      if (stdModuleOfId(source) !== null) return source;
      if (importer !== undefined && importer !== null) {
        const importerModule = stdModuleOfId(importer);
        if (importerModule !== null) {
          if (source === "./option.js") return stdId("option");
          if (source === "./result.js") return stdId("result");
        }
      }
      const file = cleanUrl(source);
      if (!TT_FILE.test(file)) return null;
      const query = queryOf(source, file);
      if (SPECIAL_QUERY.test(query)) return null;

      if (typeof this?.resolve === "function") {
        // Package exports, aliases, and dev-server urls belong to the host resolver.
        return this.resolve(file, importer, { skipSelf: true }).then(resolved => {
          if (!resolved || resolved.external || !TT_FILE.test(cleanUrl(resolved.id))) return resolved;
          return { ...resolved, id: moduleId(cleanUrl(resolved.id), query) };
        });
      }
      if (!path.isAbsolute(file) && !file.startsWith(".")) return null;
      const resolved = path.isAbsolute(file)
        ? file
        : importer === undefined || importer === null
          ? path.resolve(file)
          : path.resolve(path.dirname(cleanUrl(importer)), file);
      return moduleId(resolved, query);
    },

    async load(id) {
      const stdModule = stdModuleOfId(id);
      if (stdModule !== null) {
        const { stdout } = await run(compiler, ["--emit-std", stdModule, "--no-banner"], {
          maxBuffer: 16 * 1024 * 1024,
        });
        return { code: stdout, map: null };
      }
      const file = sourceFileOfId(id);
      if (file === null) return null;

      const args = printArgs(file, sourcemap);

      this.addWatchFile(file);
      // Compiler metadata includes erased type imports and configuration reads.
      // Register dependencies before loading so a failed build can recover too.
      try {
        const metadata = await run(compiler, ["--dependencies", file], { maxBuffer: 16 * 1024 * 1024 });
        const dependencies = JSON.parse(metadata.stdout);
        dependenciesByModule.set(id, new Set(dependencies.map(nativePath)));
        for (const dependency of dependencies) if (dependency !== file) this.addWatchFile(dependency);
        const { stdout } = await run(compiler, args, { maxBuffer: 16 * 1024 * 1024 });
        return detachInlineSourceMap(stdout, file);
      } catch (error) {
        // ttc reports `file:line:col: message` on stderr; surface that as
        // the build error so the host shows the compiler's diagnostic.
        const detail = String(error.stderr ?? error.message).trim();
        this.error(detail.replace(/^ttc:\s*/, ""));
        return null;
      }
    },

    watchChange(file) {
      if (!devServer) return;
      const changed = nativePath(file);
      for (const environment of Object.values(devServer.environments ?? { client: devServer })) {
        for (const [id, dependencies] of dependenciesByModule) {
          if (!dependencies.has(changed)) continue;
          const module = environment.moduleGraph.getModuleById(id);
          if (module) environment.moduleGraph.invalidateModule(module);
        }
      }
    },
    vite: {
      config() {
        const scanner = this?.meta?.rolldownVersion
          ? { rolldownOptions: { plugins: [rolldownScanPlugin] } }
          : { esbuildOptions: { plugins: [esbuildScanPlugin] } };
        return { optimizeDeps: { extensions: [".tt", ".ttx"], ...scanner } };
      },
      configureServer(server) { devServer = server; },
      handleHotUpdate(context) {
        const modules = new Set(context.modules);
        const changed = nativePath(context.file);
        for (const [id, dependencies] of dependenciesByModule) {
          if (!dependencies.has(changed)) continue;
          const module = context.server.moduleGraph.getModuleById(id);
          if (module) {
            context.server.moduleGraph.invalidateModule(module);
            modules.add(module);
          }
        }
        return [...modules];
      },
    },
    esbuild: {
      setup(build) {
        build.onResolve({ filter: /\.ttx?$/ }, async args => {
          if (args.pluginData?.ttResolving || path.isAbsolute(args.path) || args.path.startsWith(".")) return;
          const resolved = await build.resolve(args.path, { importer: args.importer, resolveDir: args.resolveDir, kind: args.kind, pluginData: { ttResolving: true } });
          if (resolved.errors.length || resolved.external || !/\.ttx?$/.test(resolved.path)) return resolved;
          return { path: moduleId(resolved.path, ""), namespace: "@openload28/unplugin-tt" };
        });
      },
      // esbuild resolves and loads through its own filters, and its `load`
      // may only return JavaScript — so narrow the filters to our ids and
      // name the loader for the TypeScript ttc emits.
      onResolveFilter: /(\.ttx?|^@tt\/(?:std(?:\/(?:option|result))?|runtime)$|\.\/(?:option|result)\.js$)/,
      onLoadFilter: /(\.tt\?(?:[^#]*&)?lang\.ts|\.ttx\?(?:[^#]*&)?lang\.tsx|^virtual:unplugin-tt\/std\/(?:types|option|result|runtime)\.ts)$/,
      loader: (_code, id) => (id.endsWith(TSX_SUFFIX) ? "tsx" : "ts"),
    },
  };
};

export const unplugin = /* #__PURE__ */ createUnplugin(unpluginFactory);

export default unplugin;

export const vitePlugin = unplugin.vite;
export const rollupPlugin = unplugin.rollup;
export const rolldownPlugin = unplugin.rolldown;
export const webpackPlugin = unplugin.webpack;
export const rspackPlugin = unplugin.rspack;
export const esbuildPlugin = unplugin.esbuild;
export const farmPlugin = unplugin.farm;
