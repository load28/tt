/* How an unusable compiler is reported.
 *
 * A `ttc` that is not there and a `ttc` that is there but cannot be
 * started leave the editor in the same state — no diagnostics at all — and
 * both are fixed by the person at the keyboard. Only the first used to say
 * so: the second failed with `EACCES` and fell into the generic "failed"
 * branch, which logs to a channel nobody has open and publishes an empty
 * diagnostics list, so the Problems panel simply stayed clean (TASK-340). */
import * as assert from "node:assert/strict";
import { after, test } from "node:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import * as engine from "../engine";
import { runCheck, unusableCompiler } from "../ttc";

after(() => engine.shutdownEngineServer());

const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-unusable-"));

test("a spawn failure is read for what the user has to fix", () => {
  assert.equal(unusableCompiler({ code: "ENOENT" }), "missing");
  assert.equal(unusableCompiler({ code: "EACCES" }), "not-executable");
  assert.equal(unusableCompiler({ code: "EISDIR" }), "not-executable");
  assert.equal(unusableCompiler({ code: "ENOEXEC" }), "not-executable");
  // The compiler ran and reported something — that is a diagnostic, not a
  // broken installation.
  assert.equal(unusableCompiler({ code: 1 }), null);
  assert.equal(unusableCompiler(null), null);
});

test("a compiler that is not there is reported as missing", async () => {
  const result = await runCheck(
    path.join(dir, "nowhere", "ttc"),
    "export const x = 1;\n",
    "a.tt",
    false,
  );
  assert.deepEqual(
    { kind: result.kind, reason: "reason" in result ? result.reason : null },
    { kind: "not-found", reason: "missing" },
  );
});

test("a compiler that cannot be started is reported, not swallowed", async () => {
  const notExecutable = path.join(dir, "ttc");
  fs.writeFileSync(notExecutable, "#!/bin/sh\nexit 0\n");
  fs.chmodSync(notExecutable, 0o644);

  const result = await runCheck(
    notExecutable,
    "export const x = 1;\n",
    "b.tt",
    false,
  );
  assert.deepEqual(
    { kind: result.kind, reason: "reason" in result ? result.reason : null },
    { kind: "not-found", reason: "not-executable" },
  );
});
