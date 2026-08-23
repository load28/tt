import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const readJson = (url) => JSON.parse(readFileSync(url, "utf8"));
const main = readJson(new URL("../tt-lang/package.json", import.meta.url));
const platforms = readJson(new URL("../tt-lang/platforms.json", import.meta.url));
const unplugin = readJson(new URL("../../integrations/unplugin/package.json", import.meta.url));
const initializer = readJson(new URL("../../packages/create-tt/package.json", import.meta.url));

test("every published npm package belongs to the load28 scope", () => {
  const names = [
    main.name,
    ...Object.values(platforms).map((target) => target.package),
    unplugin.name,
    initializer.name,
  ];
  assert.ok(names.every((name) => name.startsWith("@load28/")));
});

test("static packages publish publicly to npmjs", () => {
  for (const manifest of [main, unplugin, initializer]) {
    assert.deepEqual(manifest.publishConfig, {
      access: "public",
      registry: "https://registry.npmjs.org/",
    });
  }
});
