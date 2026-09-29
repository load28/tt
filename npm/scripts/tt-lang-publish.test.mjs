import assert from "node:assert/strict";
import { writeFileSync } from "node:fs";
import { join } from "node:path";
import { createRequire } from "node:module";
import test from "node:test";

import { testDir } from "../../scripts/test-dirs.cjs";

const require = createRequire(import.meta.url);
const { assertPublishable } = require("../tt-lang/assert-publishable.js");

test("the main package cannot publish a machine-local development stamp", () => {
  const clean = testDir("tt-publish-clean-");
  assert.doesNotThrow(() => assertPublishable(clean));

  const local = testDir("tt-publish-local-");
  writeFileSync(join(local, "tt-dev.local.json"), '{"root":"/private/source"}\n');
  assert.throws(
    () => assertPublishable(local),
    /refusing to publish a local development package/,
  );
});
