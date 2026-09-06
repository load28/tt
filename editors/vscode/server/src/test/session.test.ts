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

test("an explicit compiler restart settles in-flight requests immediately", { skip, timeout: 2000 }, async () => {
  const pending = engine.engineRequest(COMPILER, "check", { text: "", filename: "a.tt" }, 15000);
  engine.shutdownEngineServer();
  assert.equal(await pending, null);
  const answer = await check(COMPILER);
  assert.ok(answer && "result" in answer);
});

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

test("a second compiler keeps its own session instead of ending the first", { skip }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-two-compilers-"));
  const copy = path.join(dir, "ttc");
  fs.copyFileSync(COMPILER, copy);
  fs.chmodSync(copy, 0o755);

  const first = await check(COMPILER);
  assert.ok(first && "result" in first, "the first compiler answers");
  const second = await check(copy);
  assert.ok(second && "result" in second, "so does the second");
  // `tt.compilerPath` is resource-scoped, so alternating between two
  // documents alternates the compiler. Neither session may end the other.
  for (let round = 0; round < 3; round += 1) {
    assert.ok((await check(COMPILER)) !== null, `round ${round}: first still serves`);
    assert.ok((await check(copy)) !== null, `round ${round}: second still serves`);
  }
});

test("a session start that opens documents elsewhere cannot kill its own session", { skip }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-session-start-"));
  const copy = path.join(dir, "ttc");
  fs.copyFileSync(COMPILER, copy);
  fs.chmodSync(copy, 0o755);
  const file = path.join(dir, "a.tt");
  fs.writeFileSync(file, "variant S { A, B }\n");

  // What `server.ts` does: re-send every open buffer when a session starts.
  // The callback is told which session started; a stale "current compiler"
  // here used to shut that session down and leave the caller writing to a
  // dead pipe, which took the whole language server with it.
  const started: string[] = [];
  engine.setOnSessionStart((compiler) => {
    started.push(compiler);
    engine.openDocument(copy, file, "variant S { A, B }\n");
  });
  try {
    await check(copy);
    const answer = await check(COMPILER);
    assert.ok(answer && "result" in answer, `the fresh session answers: ${JSON.stringify(answer)}`);
    assert.deepEqual(started, [copy, COMPILER], "each session start names its own compiler");
  } finally {
    engine.setOnSessionStart(null);
  }
});
