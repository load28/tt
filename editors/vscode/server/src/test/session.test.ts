/* How the engine session decides a compiler cannot serve it.
 *
 * Two failed spawns without a single answer mean "this ttc has no
 * `--server`", and the session stops paying for a process per keystroke.
 * The verdict is about *that* compiler: another path — a `tt.compilerPath`
 * that arrived after the first documents, a package installed since — has
 * failed at nothing and must still be tried (TASK-255). */
import * as assert from "node:assert/strict";
import { after, test } from "node:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import * as engine from "../engine";
import { COMPILER, compilerAvailable } from "./toolchain";

const skip = compilerAvailable() ? false : "no ttc — none built, installed, or on PATH";

after(() => engine.shutdownEngineServer());

/** A path with nothing behind it — spawning it always fails. */
function missingCompiler(name: string): string {
  return path.join(os.tmpdir(), `tt-no-such-compiler-${name}`, "ttc");
}

async function check(compiler: string): Promise<engine.EngineAnswer> {
  return engine.engineRequest(compiler, "check", { text: "", filename: "a.tt" }, 5000);
}

test("a compiler that cannot serve is given up on, and only that one", { skip }, async () => {
  engine.retryEngineServer();
  const missing = missingCompiler("a");

  assert.equal(await check(missing), null, "a missing compiler answers nothing");
  assert.equal(await check(missing), null, "and again — that is the second strike");
  assert.equal(await check(missing), null, "now it is not even spawned");

  // The real compiler inherits none of that: the verdict was about the path
  // that failed, and this one has not failed at anything.
  const answer = await check(COMPILER);
  assert.ok(answer && "result" in answer, `the working compiler still serves: ${JSON.stringify(answer)}`);
});

test("an environment change re-arms a compiler that struck out", { skip }, async () => {
  engine.retryEngineServer();
  // A file that exists but is not a compiler: spawning succeeds on some
  // platforms and the child dies immediately, which is the same "no answer".
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-session-"));
  const notACompiler = path.join(dir, "ttc");
  fs.writeFileSync(notACompiler, "");

  await check(notACompiler);
  await check(notACompiler);
  assert.equal(await check(notACompiler), null);

  // The editor learned something new (settings changed, a compiler appeared
  // on disk); the next request gets to find out for itself.
  engine.retryEngineServer();
  const answer = await check(COMPILER);
  assert.ok(answer && "result" in answer, "the session is usable again");
});
