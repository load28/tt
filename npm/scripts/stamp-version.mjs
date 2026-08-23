/* --------------------------------------------------------------------------
 * stamp-version.mjs — write the release version into published TT packages.
 *
 *   node stamp-version.mjs <version> [--unplugin-version <version>]
 *                                  [--vscode-version <major.minor.patch>]
 *
 * The repository keeps the placeholder 0.0.0-dev so there is exactly one
 * version source of truth (Cargo.toml); the release workflow stamps the tag
 * version into the main package and its optionalDependencies just before
 * publishing.
 * ----------------------------------------------------------------------- */
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath } from "node:url";

const [version, ...options] = process.argv.slice(2);
if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
  console.error(
    "usage: stamp-version.mjs <semver> [--unplugin-version <semver>] " +
      "[--vscode-version <major.minor.patch>]",
  );
  process.exit(1);
}

const extraVersions = new Map();
for (let index = 0; index < options.length; index += 2) {
  const option = options[index];
  const value = options[index + 1];
  if (!value || !["--unplugin-version", "--vscode-version"].includes(option)) {
    console.error(`invalid stamp-version option: ${option ?? "<missing>"}`);
    process.exit(1);
  }
  extraVersions.set(option, value);
}

const manifest = path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "tt-lang", "package.json");
const pkg = JSON.parse(fs.readFileSync(manifest, "utf8"));
pkg.version = version;
for (const dep of Object.keys(pkg.optionalDependencies)) {
  pkg.optionalDependencies[dep] = version;
}
fs.writeFileSync(manifest, JSON.stringify(pkg, null, 2) + "\n");
console.log(`stamped ${version} into ${manifest}`);

const initializerManifest = path.join(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
  "..",
  "packages",
  "create-tt",
  "package.json",
);
const initializer = JSON.parse(fs.readFileSync(initializerManifest, "utf8"));
initializer.version = version;
fs.writeFileSync(initializerManifest, JSON.stringify(initializer, null, 2) + "\n");
console.log(`stamped ${version} into ${initializerManifest}`);

stampOptionalManifest(
  "--unplugin-version",
  path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "integrations", "unplugin", "package.json"),
  /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/,
);
stampOptionalManifest(
  "--vscode-version",
  path.join(path.dirname(fileURLToPath(import.meta.url)), "..", "..", "editors", "vscode", "package.json"),
  /^\d+\.\d+\.\d+$/,
);

function stampOptionalManifest(option, manifestPath, pattern) {
  const targetVersion = extraVersions.get(option);
  if (!targetVersion) return;
  if (!pattern.test(targetVersion)) {
    console.error(`invalid ${option} value: ${targetVersion}`);
    process.exit(1);
  }
  const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
  manifest.version = targetVersion;
  fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
  console.log(`stamped ${targetVersion} into ${manifestPath}`);
}
