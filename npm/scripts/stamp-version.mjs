/* --------------------------------------------------------------------------
 * stamp-version.mjs — write the release version into published TT packages.
 *
 *   node stamp-version.mjs <version> [--unplugin-version <version>]
 *                                  [--vscode-version <major.minor.patch>]
 *
 * Release workflows stamp the compiler version into the main package and its
 * optionalDependencies. They may also derive independent integration versions.
 * ----------------------------------------------------------------------- */
import * as fs from "node:fs";
import * as path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const REPOSITORY_ROOT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..", "..");

export function stampVersion(version, options = [], repositoryRoot = REPOSITORY_ROOT) {
  if (!version || !/^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/.test(version)) {
    throw new Error(`invalid semver: ${version}`);
  }
  const extraVersions = parseOptions(options);
  const manifest = path.join(repositoryRoot, "npm", "tt-lang", "package.json");
  const pkg = JSON.parse(fs.readFileSync(manifest, "utf8"));
  pkg.version = version;
  for (const dep of Object.keys(pkg.optionalDependencies)) pkg.optionalDependencies[dep] = version;
  fs.writeFileSync(manifest, JSON.stringify(pkg, null, 2) + "\n");
  console.log(`stamped ${version} into ${manifest}`);

  const initializerManifest = path.join(repositoryRoot, "packages", "create-tt", "package.json");
  const initializer = JSON.parse(fs.readFileSync(initializerManifest, "utf8"));
  initializer.version = version;
  fs.writeFileSync(initializerManifest, JSON.stringify(initializer, null, 2) + "\n");
  console.log(`stamped ${version} into ${initializerManifest}`);

  stampOptionalManifest(
    "--unplugin-version",
    path.join(repositoryRoot, "integrations", "unplugin", "package.json"),
    /^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$/,
  );
  stampOptionalManifest(
    "--vscode-version",
    path.join(repositoryRoot, "editors", "vscode", "package.json"),
    /^\d+\.\d+\.\d+$/,
  );

  function stampOptionalManifest(option, manifestPath, pattern) {
    const targetVersion = extraVersions.get(option);
    if (!targetVersion) return;
    if (!pattern.test(targetVersion)) throw new Error(`invalid ${option} value: ${targetVersion}`);
    const manifest = JSON.parse(fs.readFileSync(manifestPath, "utf8"));
    manifest.version = targetVersion;
    fs.writeFileSync(manifestPath, JSON.stringify(manifest, null, 2) + "\n");
    console.log(`stamped ${targetVersion} into ${manifestPath}`);
  }
}

function parseOptions(options) {
  const extraVersions = new Map();
  for (let index = 0; index < options.length; index += 2) {
    const option = options[index];
    const value = options[index + 1];
    if (!value || !["--unplugin-version", "--vscode-version"].includes(option)) {
      throw new Error(`invalid stamp-version option: ${option ?? "<missing>"}`);
    }
    extraVersions.set(option, value);
  }
  return extraVersions;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const [version, ...options] = process.argv.slice(2);
    if (!version) throw new Error("usage: stamp-version.mjs <semver> [options]");
    stampVersion(version, options);
  } catch (error) {
    console.error(`stamp-version: ${error.message}`);
    process.exit(1);
  }
}
