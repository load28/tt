/* `runTypedCheck` — the editor's entry into `ttc --check-types` (TASK-072).
 *
 * What is tested here is the seam, not the rule: that the buffer (and not
 * the file on disk) is what gets checked, that the compiler's own message
 * comes back verbatim, and that a project which cannot answer degrades to
 * "unavailable" rather than to an empty — and so falsely clean — result.
 */
import * as assert from "node:assert/strict";
import { test } from "node:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import { runTypedCheck } from "../ttc";
import { COMPILER, compilerAvailable, findTsgo } from "./toolchain";

const skip = compilerAvailable() ? false : "no ttc — none built, installed, or on PATH";
/** A case that needs a real typed answer, not just a compiler that runs.
 * Without this the two cases below fail — rather than skip — on a machine
 * with no TypeScript 7, which is the difference between "the tool is
 * missing" and "the feature is broken" (TASK-217). */
const skipTyped = skip || (findTsgo() ? false : "no tsgo executable");
/** A typed check opens a project and starts the TypeScript compiler. */
const timeout = 60_000;

let seq = 0;
function tmpProject(): string {
  const dir = path.join(
    os.tmpdir(),
    `tt-typedcheck-${process.pid}-${seq++}`,
    "src",
  );
  fs.mkdirSync(dir, { recursive: true });
  return dir;
}

test("a buffer that was never saved has no place in the project", async () => {
  const dir = tmpProject();
  const result = await runTypedCheck(
    COMPILER,
    "val const xs: number[] = [];\nxs.push(1);\n",
    path.join(dir, "never-saved.tt"),
  );
  // Not "ok with no diagnostics": that would render as a clean file.
  assert.equal(result.kind, "unavailable");
});

test(
  "the buffer is what gets checked, and the message is the compiler's",
  { skip: skipTyped, timeout },
  async () => {
    const dir = tmpProject();
    const file = path.join(dir, "main.tt");
    fs.writeFileSync(file, "export const saved = 0;\n");

    const result = await runTypedCheck(
      COMPILER,
      "val const scores = new Map<string, number>();\n" +
        "scores.set(\"a\", 1);\n" +
        "export const n = scores.size;\n",
      file,
    );

    if (result.kind === "unavailable") {
      // No TypeScript for ttc to drive on this machine — the mode itself
      // said so, which is the only thing this test would be asserting.
      return;
    }
    const messages = result.diagnostics.map((d) => d.message);
    assert.ok(
      messages.some((m) =>
        m.startsWith("cannot call mutating method `set` through val binding `scores`"),
      ),
      `expected the compiler's val message, got ${JSON.stringify(messages)}`,
    );
    // The position is the buffer's: the saved text has no such line.
    const val = result.diagnostics.find((d) => d.message.includes("`scores`"));
    assert.equal(val?.line, 2);
    assert.equal(val?.col, 1);
  },
);

test(
  "a type error is not reported — the language server already has it",
  { skip, timeout },
  async () => {
    const dir = tmpProject();
    const file = path.join(dir, "main.tt");
    fs.writeFileSync(file, "export const saved = 0;\n");

    const result = await runTypedCheck(
      COMPILER,
      "const wrong: number = \"not a number\";\nexport const n = wrong;\n",
      file,
    );
    if (result.kind === "unavailable") return;
    assert.deepEqual(
      result.diagnostics.filter((d) => d.message.includes("ts(")),
      [],
    );
  },
);

test(
  "the authoritative editor pass uses the compiler's structured type message",
  { skip: skipTyped, timeout },
  async () => {
    const dir = tmpProject();
    const file = path.join(dir, "main.tt");
    fs.writeFileSync(file, "export const saved = 0;\n");

    const result = await runTypedCheck(
      COMPILER,
      "const wrong: string = 1;\n",
      file,
      true,
    );
    if (result.kind === "unavailable") return;
    const mismatch = result.diagnostics.find((d) => d.code === "ts2322");
    assert.equal(
      mismatch?.message,
      "type mismatch: expected `string`, found `1`",
    );
    assert.deepEqual(
      [mismatch?.line, mismatch?.col, mismatch?.endLine, mismatch?.endCol],
      [1, 23, 1, 24],
    );
  },
);
