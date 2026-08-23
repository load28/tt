/* --------------------------------------------------------------------------
 * make-platform-package.mjs — assemble one @load28/tt-lang-<os>-<cpu> npm package.
 *
 *   node make-platform-package.mjs <os>-<cpu> <path/to/ttc[.exe]> <out-dir> \
 *        [version]
 *
 * Writes <out-dir>/@load28/tt-lang-<os>-<cpu>/ with package.json, the binary under
 * bin/, and a one-line README. The release workflow runs this once per build
 * matrix entry; version defaults to 0.0.0-dev for local smoke tests.
 * ----------------------------------------------------------------------- */
import * as fs from "node:fs";
import * as path from "node:path";

const TARGETS = JSON.parse(
  fs.readFileSync(new URL("../tt-lang/platforms.json", import.meta.url), "utf8"),
);

const [key, binarySource, outDir, version = "0.0.0-dev"] = process.argv.slice(2);
const target = TARGETS[key];
if (!target || !binarySource || !outDir) {
  console.error(
    `usage: make-platform-package.mjs <${Object.keys(TARGETS).join("|")}> ` +
      `<path/to/ttc[.exe]> <out-dir> [version]`,
  );
  process.exit(1);
}

const name = target.package;
const exe = target.os === "win32" ? "ttc.exe" : "ttc";
const root = path.join(outDir, name);

fs.mkdirSync(path.join(root, "bin"), { recursive: true });
fs.copyFileSync(binarySource, path.join(root, "bin", exe));
if (target.os !== "win32") fs.chmodSync(path.join(root, "bin", exe), 0o755);

fs.writeFileSync(
  path.join(root, "package.json"),
  JSON.stringify(
    {
      name,
      version,
      description: `Prebuilt ttc binary (tt compiler) for ${key}. Install @load28/tt-lang instead of this package.`,
      license: "MIT",
      repository: { type: "git", url: "https://github.com/load28/tt", directory: "npm/tt-lang" },
      publishConfig: { access: "public", registry: "https://registry.npmjs.org/" },
      preferUnplugged: true,
      os: [target.os],
      cpu: [target.cpu],
    },
    null,
    2,
  ) + "\n",
);
fs.writeFileSync(
  path.join(root, "README.md"),
  `# ${name}\n\nPrebuilt \`ttc\` binary for ${key}. This is an internal platform package —\ninstall [\`@load28/tt-lang\`](https://www.npmjs.com/package/@load28/tt-lang) instead.\n`,
);

console.log(root);
