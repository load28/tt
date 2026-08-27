/* --------------------------------------------------------------------------
 * build-ts-preview-vsix — package the TypeScript preview VS Code extension
 * that matches this repository's pinned TypeScript nightly (TASK-258).
 *
 *   node npm/scripts/build-ts-preview-vsix.mjs <out-dir>
 *
 * The marketplace's "TypeScript Native Preview" predates content mappers,
 * so tt ships an editor counterpart built from the exact commit the pinned
 * `typescript` nightly records (npm metadata `gitHead`):
 *
 *   1. read the pin from the repository's package.json
 *   2. `npm view` its gitHead, sparse-clone microsoft/TypeScript there
 *   3. build `packages/vscode-typescript` (tsc + esbuild, upstream scripts)
 *   4. rename the identity to `load28.tt-typescript-preview` — Apache-2.0
 *      redistribution with LICENSE/NOTICE kept, never Microsoft's publisher
 *   5. for each supported platform: copy the same-version platform npm
 *      package's `lib/` (tsgo executable + default libs — the extension
 *      activates from its packaged executable, TASK-257 issue log) and
 *      `vsce package --target` it
 *
 * The extension version derives from the pin (`7.1.0-dev.YYYYMMDD.N` →
 * `0.YYYYMMDD.N`), so a re-run of the same pin reproduces the same VSIX
 * and the version only moves when the pin moves.
 * ----------------------------------------------------------------------- */
import { execFileSync } from "node:child_process";
import { chmodSync, cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** VS Code platform targets and the npm platform package serving each. */
export const PLATFORMS = [
  { target: "linux-x64", npmPackage: "@typescript/typescript-linux-x64" },
  { target: "linux-arm64", npmPackage: "@typescript/typescript-linux-arm64" },
  { target: "darwin-x64", npmPackage: "@typescript/typescript-darwin-x64" },
  { target: "darwin-arm64", npmPackage: "@typescript/typescript-darwin-arm64" },
  { target: "win32-x64", npmPackage: "@typescript/typescript-win32-x64" },
];

/** The extension's identity: the upstream id, on purpose.
 *
 * VS Code's built-in TypeScript extension yields its semantic service only
 * when `useTsgo` is set AND an extension from its hardcoded list —
 * `typescriptteam.vscode-typescript`, `typescriptteam.native-preview` — is
 * installed (TASK-259). A renamed id therefore leaves two servers running
 * and the built-in reports TS2307 on every `.tt` import. This build is a
 * stopgap the marketplace release supersedes (same id, higher version →
 * auto-update replaces it), never something tt distributes as its own
 * product; the description carries the provenance. */
export const EXTENSION_IDENTITY = {
  publisher: "TypeScriptTeam",
  name: "native-preview",
  displayName: "TypeScript (Native Preview)",
};

/**
 * The extension version a TypeScript nightly pin maps to:
 * `7.1.0-dev.20260826.1` → `0.20260826.1`. Deterministic on purpose — the
 * version moves only when the pin moves.
 */
export function extensionVersionFor(pin) {
  const match = /^\d+\.\d+\.\d+-dev\.(\d{8})\.(\d+)$/.exec(pin);
  if (!match) {
    throw new Error(
      `cannot derive an extension version from typescript pin "${pin}" — expected an X.Y.Z-dev.YYYYMMDD.N nightly`,
    );
  }
  return `0.${match[1]}.${match[2]}`;
}

/** The VSIX file name for one platform build. */
export function vsixName(version, target) {
  return `tt-typescript-preview-${version}-${target}.vsix`;
}

/** The pinned typescript version in this repository's package.json. */
export function pinnedTypeScript(rootPackageJson) {
  const pin = JSON.parse(rootPackageJson).devDependencies?.typescript;
  if (typeof pin !== "string") {
    throw new Error("the repository package.json pins no typescript version");
  }
  return pin;
}

function run(command, args, options = {}) {
  return execFileSync(command, args, { encoding: "utf8", stdio: ["ignore", "pipe", "inherit"], ...options });
}

function main() {
  const outDir = process.argv[2];
  if (!outDir) {
    console.error("usage: node npm/scripts/build-ts-preview-vsix.mjs <out-dir>");
    process.exit(1);
  }
  const repoRoot = path.resolve(fileURLToPath(import.meta.url), "../../..");
  const pin = pinnedTypeScript(readFileSync(path.join(repoRoot, "package.json"), "utf8"));
  const version = extensionVersionFor(pin);
  const gitHead = run("npm", ["view", `typescript@${pin}`, "gitHead"]).trim();
  if (!/^[0-9a-f]{40}$/.test(gitHead)) {
    throw new Error(`typescript@${pin} records no usable gitHead: "${gitHead}"`);
  }
  console.log(`typescript ${pin} → gitHead ${gitHead} → extension ${version}`);

  const work = mkdtempSync(path.join(tmpdir(), "tt-ts-preview-"));
  try {
    // The exact source the pinned nightly was built from. A blobless clone
    // fetches only the files the checkout touches.
    const checkout = path.join(work, "TypeScript");
    run("git", ["clone", "--filter=blob:none", "--no-checkout", "https://github.com/microsoft/TypeScript", checkout]);
    run("git", ["-C", checkout, "checkout", gitHead]);

    // Upstream's own build, from upstream's own lockfile.
    run("npm", ["ci"], { cwd: checkout });
    const extensionDir = path.join(checkout, "packages/vscode-typescript");
    run("npm", ["run", "build"], { cwd: extensionDir });

    // Our identity, upstream's provenance. LICENSE/NOTICE ship as-is.
    const manifestPath = path.join(extensionDir, "package.json");
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    manifest.publisher = EXTENSION_IDENTITY.publisher;
    manifest.name = EXTENSION_IDENTITY.name;
    manifest.displayName = EXTENSION_IDENTITY.displayName;
    manifest.description = `TypeScript ${pin} language server with content mapper support, packaged by tt until the marketplace preview catches up. Built unmodified from microsoft/TypeScript@${gitHead.slice(0, 12)}.`;
    manifest.version = version;
    manifest.bundledTypeScriptVersion = pin;
    writeFileSync(manifestPath, JSON.stringify(manifest, undefined, 4));
    cpSync(path.join(checkout, "NOTICE.txt"), path.join(extensionDir, "NOTICE.txt"));

    mkdirSync(outDir, { recursive: true });
    for (const { target, npmPackage } of PLATFORMS) {
      // The same-version platform package carries lib/: the tsgo
      // executable and the default lib files, in the layout the extension
      // resolves first (TASK-257: no executable, no activation).
      const platformDir = path.join(work, target);
      mkdirSync(platformDir, { recursive: true });
      const tarball = run("npm", ["pack", `${npmPackage}@${pin}`, "--pack-destination", platformDir]).trim();
      run("tar", ["-xzf", path.join(platformDir, tarball), "-C", platformDir]);

      const libDir = path.join(extensionDir, "lib");
      rmSync(libDir, { recursive: true, force: true });
      cpSync(path.join(platformDir, "package/lib"), libDir, { recursive: true });
      const exe = target === "win32-x64" ? "tsc.exe" : "tsc";
      chmodSync(path.join(libDir, exe), 0o755);

      const vsix = path.resolve(outDir, vsixName(version, target));
      run("npx", ["--yes", "@vscode/vsce@3.9.2", "package", "--no-dependencies", "--allow-unused-files-pattern", "--target", target, "--out", vsix], { cwd: extensionDir });
      console.log(`packaged ${vsix}`);
    }
  } finally {
    rmSync(work, { recursive: true, force: true });
  }
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  main();
}
