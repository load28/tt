/* --------------------------------------------------------------------------
 * @openload28/unplugin-tt — `.tt` modules for every bundler unplugin supports.
 *
 * The plugin resolves `.tt` specifiers itself and compiles each file with
 * `ttc` on the way in, so a project needs no intermediate `.ts` tree: the
 * bundler reads the sources directly.
 *
 * Modules are compiled by one `ttc --server` session per build — the
 * answer to each request is exactly what `ttc -p` prints — so the
 * TypeScript project the compiler refines its output with opens once, not
 * once per module (`compiler-server.js`).
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
 *   while hosts that strip the query still find the file on disk. esbuild
 *   loads these modules through its own `onLoad`, which names the loader.
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

import { TS_SUFFIX, TSX_SUFFIX, TT_FILE, SPECIAL_QUERY, SCANNED_FILE, TT_MODULE_ID, fileOf, queryOf, moduleId, sourceFileOfId } from "./module-id.js";
import { CompilerServer } from "./compiler-server.js";

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

const isWithin = (directory, file) => {
  const relative = path.relative(directory, file);
  return relative === "" || (relative.split(/[\\/]/)[0] !== ".." && !path.isAbsolute(relative));
};

/**
 * Whether a module whose compile read `dependencies` is stale after the
 * watcher reported `event` for `changed`: a file it read changed, or an
 * entry appeared in or left a directory it listed.
 */
const dependsOn = (dependencies, changed, event) =>
  dependencies.files.has(changed) ||
  ((event === "create" || event === "delete") && dependencies.directories.has(path.dirname(changed)));

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

/** `code` with `map` written into it as a `data:` URL, the form esbuild reads. */
const inlineSourceMap = (code, map) =>
  map === null
    ? code
    : `${code}//# sourceMappingURL=data:application/json;charset=utf-8;base64,${Buffer.from(JSON.stringify(map)).toString("base64")}\n`;

const PLUGIN_NAME = "@openload28/unplugin-tt";

/**
 * @typedef {object} Options
 * @property {string} [compiler] Path to the ttc binary (default: the
 *   installed `@openload28/tt-lang` package's binary, falling back to `"ttc"` on PATH).
 * @property {boolean} [verify] Run ttc's output self-check (default: true).
 * @property {boolean} [sourcemap] Ask ttc for a source map and hand it to the
 *   host, so a stack trace and a debugger point at the `.tt` (default: true).
 */

/** @type {import("unplugin").UnpluginFactory<Options | undefined>} */
export const unpluginFactory = (options = {}, meta = {}) => {
  const compiler = options.compiler ?? defaultCompiler();
  const verify = options.verify ?? true;
  const sourcemap = options.sourcemap ?? true;
  const dependenciesByModule = new Map();
  const server = new CompilerServer(compiler);
  let devServer;

  /**
   * `ttc -p --rewrite-imports off <file>`, asked of the server. ttc writes
   * the map into the output as a data: URL, the form `-p` prints; it is
   * split back out so the host gets a real map object and composes it
   * with its own transforms. A failed compile throws what `-p` would have
   * written on stderr.
   */
  const print = async (file, withMap) => {
    const { code, messages } = await server.request("print", {
      path: file,
      rewriteImports: "off",
      sourceMap: withMap ? "inline" : "off",
      verify,
    });
    if (code === null) throw new Error(messages.map((message) => `${message}\n`).join(""));
    return code;
  };

  /**
   * What `ttc --dependencies` prints for the module's file, asked of the
   * server and kept for invalidation: the files its compile reads and the
   * directories it lists.
   */
  const dependenciesOf = async (id, file) => {
    const dependencies = await server.request("dependencies", { path: file });
    dependenciesByModule.set(id, {
      files: new Set(dependencies.files.map(nativePath)),
      directories: new Set(dependencies.directories.map(nativePath)),
    });
    return dependencies;
  };

  /**
   * Registers a module's dependencies with the host's watcher, each kind
   * through the API the host defines for it.
   *
   * - Vite's dev server treats a file given to `addWatchFile` as an import
   *   of the module and resolves it, so the paths go to the server's own
   *   watcher instead (it already watches everything under the root), and
   *   `watchChange` invalidates the modules that read them.
   * - webpack and Rspack take a directory as a context dependency.
   * - Rollup's `addWatchFile` accepts a file or a directory; Rolldown,
   *   Vite's build and Farm take the same call.
   * - esbuild loads these modules through its own `onLoad`, whose result
   *   names both kinds.
   */
  function watchDependencies(file, { files, directories }) {
    if (devServer) {
      const outside = [...files, ...directories].filter((dependency) => !isWithin(devServer.config.root, dependency));
      if (outside.length > 0) devServer.watcher.add(outside);
      return;
    }
    for (const dependency of files) if (dependency !== file) this.addWatchFile(dependency);
    const native = meta.framework === "webpack" || meta.framework === "rspack" ? this.getNativeBuildContext?.() : undefined;
    for (const directory of directories) {
      if (native?.loaderContext) native.loaderContext.addContextDependency(directory);
      else this.addWatchFile(directory);
    }
  }

  /**
   * One module's compile: its dependencies, then what `ttc -p` prints for
   * it. A failed compile still answers the dependencies it could learn, so
   * the host keeps watching them and builds again once the source is fixed.
   */
  const compileModule = async (id, file) => {
    let dependencies = { files: [], directories: [] };
    try {
      dependencies = await dependenciesOf(id, file);
      return { dependencies, output: detachInlineSourceMap(await print(file, sourcemap), file) };
    } catch (error) {
      // ttc reports `file:line:col: message`; that is the build error, so
      // the host shows the compiler's diagnostic.
      return { dependencies, error: error.message.trim().replace(/^ttc:\s*/, "") };
    }
  };

  const scanSource = async (id) => {
    const file = fileOf(id);
    return { code: await print(file, false), lang: file.endsWith(".ttx") ? "tsx" : "ts" };
  };

  const esbuildScanPlugin = {
    name: "@openload28/unplugin-tt:dep-scan",
    setup(build) {
      build.onLoad({ filter: SCANNED_FILE }, async (args) => {
        const { code, lang } = await scanSource(args.path);
        return { contents: code, loader: lang, resolveDir: path.dirname(fileOf(args.path)) };
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
    name: PLUGIN_NAME,
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
      const importerFile = importer === undefined || importer === null ? undefined : fileOf(importer);
      const file = fileOf(
        source,
        importerFile !== undefined && source.startsWith(".") ? path.dirname(importerFile) : undefined,
      );
      if (!TT_FILE.test(file)) return null;
      const query = queryOf(source, file);
      if (SPECIAL_QUERY.test(query)) return null;

      if (typeof this?.resolve === "function") {
        // Package exports, aliases, and dev-server urls belong to the host resolver.
        return this.resolve(file, importer, { skipSelf: true }).then(resolved => {
          if (!resolved || resolved.external) return resolved;
          const resolvedFile = fileOf(resolved.id);
          if (!TT_FILE.test(resolvedFile)) return resolved;
          return { ...resolved, id: moduleId(resolvedFile, query) };
        });
      }
      if (!path.isAbsolute(file) && !file.startsWith(".")) return null;
      const resolved = path.isAbsolute(file)
        ? file
        : importerFile === undefined
          ? path.resolve(file)
          : path.resolve(path.dirname(importerFile), file);
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

      this.addWatchFile(file);
      // Compiler metadata includes erased type imports and configuration reads.
      // They are registered before the error so a failed build can recover too.
      const { dependencies, output, error } = await compileModule(id, file);
      watchDependencies.call(this, file, dependencies);
      // Rollup-compatible hosts throw from `this.error`; webpack and Rspack
      // record the error on the module.
      if (error !== undefined) this.error(error);
      return output ?? null;
    },

    closeBundle() {
      // A watching build bundles again after this; its watcher's close ends
      // the session instead. A dev server closes its bundle once, on close.
      if (!this?.meta?.watchMode || devServer) server.close();
    },
    closeWatcher() {
      server.close();
    },

    watchChange(file, change) {
      if (!devServer) return;
      const changed = nativePath(file);
      for (const environment of Object.values(devServer.environments ?? { client: devServer })) {
        for (const [id, dependencies] of dependenciesByModule) {
          if (!dependsOn(dependencies, changed, change?.event)) continue;
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
          if (!dependsOn(dependencies, changed, "update")) continue;
          const module = context.server.moduleGraph.getModuleById(id);
          if (module) {
            context.server.moduleGraph.invalidateModule(module);
            modules.add(module);
          }
        }
        return [...modules];
      },
    },
    webpack(bundler) {
      bundler.hooks.shutdown.tap("@openload28/unplugin-tt", () => server.close());
    },
    rspack(bundler) {
      bundler.hooks.shutdown.tap("@openload28/unplugin-tt", () => server.close());
    },
    esbuild: {
      setup(build) {
        build.onDispose(() => server.close());
        build.onResolve({ filter: /\.ttx?$/ }, async args => {
          if (args.pluginData?.ttResolving || path.isAbsolute(args.path) || args.path.startsWith(".")) return;
          const resolved = await build.resolve(args.path, { importer: args.importer, resolveDir: args.resolveDir, kind: args.kind, pluginData: { ttResolving: true } });
          if (resolved.errors.length || resolved.external || !/\.ttx?$/.test(resolved.path)) return resolved;
          return { path: moduleId(resolved.path, ""), namespace: PLUGIN_NAME };
        });
        // unplugin's esbuild bridge answers a `load` without code with
        // nothing at all, which drops its errors and watch files: esbuild
        // then reports that it cannot load the path, and a watch never
        // looks at the source again. esbuild's own `onLoad` result carries
        // the diagnostic together with `watchFiles` and `watchDirs`.
        build.onLoad({ filter: TT_MODULE_ID, namespace: PLUGIN_NAME }, async (args) => {
          const id = args.path + (args.suffix ?? "");
          const file = sourceFileOfId(id);
          if (file === null) return undefined;
          const { dependencies, output, error } = await compileModule(id, file);
          const watched = {
            watchFiles: [...new Set([file, ...dependencies.files])],
            watchDirs: dependencies.directories,
          };
          if (error !== undefined) return { errors: [{ text: error }], ...watched };
          return {
            contents: inlineSourceMap(output.code, output.map),
            loader: file.endsWith(".ttx") ? "tsx" : "ts",
            resolveDir: path.dirname(file),
            ...watched,
          };
        });
      },
      // esbuild resolves and loads through its own filters, and its `load`
      // may only return JavaScript — so narrow the filters to our ids and
      // name the loader for the TypeScript ttc emits. The standard library
      // loads through the shared `load`; tt modules through `setup` above.
      onResolveFilter: /(\.ttx?|^@tt\/(?:std(?:\/(?:option|result))?|runtime)$|\.\/(?:option|result)\.js$)/,
      onLoadFilter: /^virtual:unplugin-tt\/std\/(?:types|option|result|runtime)\.ts$/,
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
