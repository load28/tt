/* --------------------------------------------------------------------------
 * @load28/unplugin-tt — `.tt` modules for every bundler unplugin supports.
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
 * - Module ids get a `.ts` suffix. ttc emits TypeScript, and the host's own
 *   TypeScript pass keys off the extension — this keeps the plugin out of
 *   that job entirely. esbuild is told the loader explicitly instead, since
 *   its `load` hook may only return JavaScript.
 *
 * Editor support is separate: `ttc --types` writes the declarations that let
 * a `.ts` file import `.tt` without the type checker complaining.
 * ----------------------------------------------------------------------- */
import { execFile } from "node:child_process";
import { createRequire } from "node:module";
import * as path from "node:path";
import { promisify } from "node:util";

import { createUnplugin } from "unplugin";

const run = promisify(execFile);

/**
 * Default compiler: the prebuilt binary from an installed `@load28/tt-lang` npm
 * package when present (spawned directly — no per-call node launcher),
 * otherwise `ttc` from PATH as before.
 */
function defaultCompiler() {
  try {
    const require = createRequire(import.meta.url);
    return require("@load28/tt-lang").binaryPath();
  } catch {
    return "ttc";
  }
}

/** Virtual suffix for ordinary `.tt` modules. */
const TS_SUFFIX = ".ts";
/** Virtual suffix for JSX-bearing `.ttx` modules. */
const TSX_SUFFIX = ".tsx";

const sourceSuffix = (file) => (file.endsWith(".ttx") ? TSX_SUFFIX : TS_SUFFIX);

const sourceFileOfId = (id) => {
  if (id.endsWith(`.ttx${TSX_SUFFIX}`)) return id.slice(0, -TSX_SUFFIX.length);
  if (id.endsWith(`.tt${TS_SUFFIX}`)) return id.slice(0, -TS_SUFFIX.length);
  return null;
};

/** The bare specifier tt sources use for the standard library. */
const STD_MODULES = new Map([
  ["@tt/std", "types"],
  ["@tt/std/option", "option"],
  ["@tt/std/result", "result"],
  ["@tt/runtime", "runtime"],
]);

/** Virtual module id for the standard library, per working directory. */
const stdId = (module) =>
  path.resolve(process.cwd(), "__tt_std__", `${module}${TS_SUFFIX}`);

const stdModuleOfId = (id) => {
  for (const module of STD_MODULES.values()) {
    if (id === stdId(module)) return module;
  }
  return null;
};

const INLINE_MAP =
  /\n\/\/# sourceMappingURL=data:application\/json;charset=utf-8;base64,([A-Za-z0-9+/=]+)\n?$/;

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
function detachInlineSourceMap(code) {
  const found = INLINE_MAP.exec(code);
  if (found === null) return { code, map: null };
  try {
    const map = JSON.parse(Buffer.from(found[1], "base64").toString("utf8"));
    return { code: code.slice(0, found.index + 1), map };
  } catch {
    // An unreadable map is not a reason to fail the build; the code is
    // still exactly what ttc produced.
    return { code, map: null };
  }
}

/**
 * @typedef {object} Options
 * @property {string} [compiler] Path to the ttc binary (default: the
 *   installed `@load28/tt-lang` package's binary, falling back to `"ttc"` on PATH).
 * @property {boolean} [verify] Run ttc's output self-check (default: true).
 * @property {boolean} [sourcemap] Ask ttc for a source map and hand it to the
 *   host, so a stack trace and a debugger point at the `.tt` (default: true).
 */

/** @type {import("unplugin").UnpluginFactory<Options | undefined>} */
export const unpluginFactory = (options = {}) => {
  const compiler = options.compiler ?? defaultCompiler();
  const verify = options.verify ?? true;
  const sourcemap = options.sourcemap ?? true;

  return {
    name: "@load28/unplugin-tt",
    // Ahead of the host's own resolution: `.tt` is not an extension it
    // knows. Rollup and esbuild ignore `enforce`, where plugin order is the
    // author's responsibility instead.
    enforce: "pre",

    resolveId(source, importer) {
      // The standard library has no file: ttc prints it on demand, so it
      // becomes a virtual module. Nothing lands in the project tree.
      const stdModule = STD_MODULES.get(source);
      if (stdModule !== undefined) return stdId(stdModule);
      if (importer !== undefined && importer !== null) {
        const importerModule = stdModuleOfId(importer);
        if (importerModule !== null) {
          if (source === "./option.js") return stdId("option");
          if (source === "./result.js") return stdId("result");
        }
      }
      if (!source.endsWith(".tt") && !source.endsWith(".ttx")) return null;

      const file = path.isAbsolute(source)
        ? source
        : importer === undefined || importer === null
          ? null
          : path.resolve(path.dirname(importer), source);
      return file === null ? null : `${file}${sourceSuffix(file)}`;
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

      const args = ["-p", "--rewrite-imports", "off"];
      if (!verify) args.push("--no-verify");
      // ttc prints the map into the output as a data: URL — the one form
      // that survives a pipe. It is split back out here so the host gets a
      // real map object and composes it with its own transforms.
      if (sourcemap) args.push("--source-map", "inline");
      args.push(file);

      try {
        const { stdout } = await run(compiler, args, { maxBuffer: 16 * 1024 * 1024 });
        this.addWatchFile(file);
        return detachInlineSourceMap(stdout);
      } catch (error) {
        // ttc reports `file:line:col: message` on stderr; surface that as
        // the build error so the host shows the compiler's diagnostic.
        const detail = String(error.stderr ?? error.message).trim();
        this.error(detail.replace(/^ttc:\s*/, ""));
        return null;
      }
    },

    esbuild: {
      // esbuild resolves and loads through its own filters, and its `load`
      // may only return JavaScript — so narrow the filters to our ids and
      // name the loader for the TypeScript ttc emits.
      onResolveFilter: /(\.ttx?|^@tt\/(?:std(?:\/(?:option|result))?|runtime)$|\.\/(?:option|result)\.js$)/,
      onLoadFilter: /(\.tt\.ts|\.ttx\.tsx|__tt_std__\/(?:types|option|result|runtime)\.ts)$/,
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
