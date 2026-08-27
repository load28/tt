import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

import {
  EXTENSION_IDENTITY,
  PLATFORMS,
  extensionVersionFor,
  pinnedTypeScript,
  vsixName,
} from "./build-ts-preview-vsix.mjs";

test("the extension version derives from the pin and only from the pin", () => {
  assert.equal(extensionVersionFor("7.1.0-dev.20260826.1"), "0.20260826.1");
  assert.equal(extensionVersionFor("8.0.0-dev.20270101.12"), "0.20270101.12");
  // A stable pin is the signal this packaging should retire, not silently
  // invent a version for (the marketplace preview carries mappers then).
  assert.throws(() => extensionVersionFor("7.1.0"), /X\.Y\.Z-dev\.YYYYMMDD\.N/);
  assert.throws(() => extensionVersionFor("7.1.0-beta"), /cannot derive/);
});

test("the packaged platforms mirror the ttc release matrix", () => {
  assert.deepEqual(
    PLATFORMS.map(p => p.target).sort(),
    ["darwin-arm64", "darwin-x64", "linux-arm64", "linux-x64", "win32-x64"],
  );
  for (const { target, npmPackage } of PLATFORMS) {
    assert.match(npmPackage, /^@typescript\/typescript-/);
    // The npm platform suffix and the VS Code target agree, so the lib
    // copied in is the one the VSIX's target runs on.
    assert.ok(npmPackage.endsWith(target), `${npmPackage} serves ${target}`);
  }
});

test("the identity is the upstream id the built-in extension yields to", () => {
  // The built-in TypeScript extension's yield list is hardcoded to the
  // typescriptteam ids (TASK-259); any other id leaves it running its own
  // semantic server, which reports TS2307 on every .tt import.
  assert.equal(EXTENSION_IDENTITY.publisher, "TypeScriptTeam");
  assert.equal(EXTENSION_IDENTITY.name, "native-preview");
  assert.equal(
    vsixName("0.20260826.1", "darwin-arm64"),
    "tt-typescript-preview-0.20260826.1-darwin-arm64.vsix",
  );
});

test("the repository pin is readable the way the script reads it", () => {
  const rootPackageJson = readFileSync(new URL("../../package.json", import.meta.url), "utf8");
  const pin = pinnedTypeScript(rootPackageJson);
  // Whatever the pin is today, the derivation accepts it — the version
  // test above is what breaks when the pin's shape changes.
  assert.match(extensionVersionFor(pin), /^0\.\d{8}\.\d+$/);
  assert.throws(() => pinnedTypeScript("{}"), /pins no typescript/);
});

test("the tt extension's lookup knows the shipped identity", () => {
  const lookup = readFileSync(
    new URL("../../editors/vscode/client/src/contentMapper.ts", import.meta.url),
    "utf8",
  );
  assert.match(lookup, /TypeScriptTeam\.native-preview/);
});
