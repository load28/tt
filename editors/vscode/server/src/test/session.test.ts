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

test("a timed-out conversation is retired and the next request restarts", { timeout: 4000 }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-session-timeout-"));
  const compiler = path.join(dir, "ttc");
  const marker = path.join(dir, "first-process-started");
  fs.writeFileSync(
    compiler,
    `#!/usr/bin/env node
const fs = require("node:fs");
const marker = ${JSON.stringify(marker)};
if (!fs.existsSync(marker)) {
  fs.writeFileSync(marker, "");
  process.stdin.resume();
} else {
  let buffer = "";
  process.stdin.setEncoding("utf8");
  process.stdin.on("data", chunk => {
    buffer += chunk;
    let newline;
    while ((newline = buffer.indexOf("\\n")) !== -1) {
      const request = JSON.parse(buffer.slice(0, newline));
      buffer = buffer.slice(newline + 1);
      process.stdout.write(JSON.stringify({id: request.id, result: {recovered: true}}) + "\\n");
    }
  });
}
`,
  );
  fs.chmodSync(compiler, 0o755);

  const first = engine.engineRequest(compiler, "first", {}, 1000);
  for (let attempt = 0; attempt < 100 && !fs.existsSync(marker); attempt += 1) {
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  assert.ok(fs.existsSync(marker), "the stalled process started");
  const started = Date.now();
  const queued = engine.engineRequest(compiler, "queued", {}, 2500);
  assert.deepEqual(await Promise.all([first, queued]), [null, null]);
  assert.ok(Date.now() - started < 1800, "queued requests settle with the timed-out session");

  const recovered = await engine.engineRequest(compiler, "next", {}, 2000);
  assert.deepEqual(recovered, { result: { recovered: true } });
});

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

function fakeCompiler(prefix: string, handle: string): string {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), prefix));
  const compiler = path.join(dir, "ttc");
  fs.writeFileSync(
    compiler,
    `#!/usr/bin/env node
let buffer = "";
const queue = [];
let busy = false;
const reply = message => process.stdout.write(JSON.stringify(message) + "\\n");
const next = () => {
  if (busy || queue.length === 0) return;
  busy = true;
  const request = queue.shift();
  const done = () => { busy = false; next(); };
  (${handle})(request, reply, done);
};
process.stdin.setEncoding("utf8");
process.stdin.on("data", chunk => {
  buffer += chunk;
  let newline;
  while ((newline = buffer.indexOf("\\n")) !== -1) {
    queue.push(JSON.parse(buffer.slice(0, newline)));
    buffer = buffer.slice(newline + 1);
  }
  next();
});
`,
  );
  fs.chmodSync(compiler, 0o755);
  return compiler;
}

test("a queued request's timeout starts when it reaches the head of the line", { timeout: 5000 }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const compiler = fakeCompiler("tt-session-head-", `(request, reply, done) => {
    const delay = request.method === "slow" ? 600 : 0;
    setTimeout(() => { reply({ id: request.id, result: { method: request.method } }); done(); }, delay);
  }`);

  const slow = engine.engineRequest(compiler, "slow", {}, 2000);
  const queued = engine.engineRequest(compiler, "queued", {}, 400);
  assert.deepEqual(await Promise.all([slow, queued]), [
    { result: { method: "slow" } },
    { result: { method: "queued" } },
  ]);
  assert.deepEqual(await engine.engineRequest(compiler, "after", {}, 400), { result: { method: "after" } });
  engine.shutdownEngineServer();
});

test("an error reply without an id settles the request at the head of the line", { timeout: 5000 }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const compiler = fakeCompiler("tt-session-null-id-", `(request, reply, done) => {
    if (request.method === "bad") reply({ id: null, error: "malformed request: boom" });
    else reply({ id: request.id, result: { method: request.method } });
    done();
  }`);

  const answers = await Promise.all([
    engine.engineRequest(compiler, "first", {}, 1000),
    engine.engineRequest(compiler, "bad", {}, 1000),
    engine.engineRequest(compiler, "second", {}, 1000),
  ]);
  assert.deepEqual(answers, [
    { result: { method: "first" } },
    { error: "malformed request: boom" },
    { result: { method: "second" } },
  ]);
  assert.deepEqual(await engine.engineRequest(compiler, "third", {}, 1000), { result: { method: "third" } });
  engine.shutdownEngineServer();
});

test("every string a request carries reaches the engine as well-formed Unicode", { timeout: 5000 }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const compiler = fakeCompiler("tt-session-well-formed-", `(request, reply, done) => {
    const lone = /[\\uD800-\\uDBFF](?![\\uDC00-\\uDFFF])|(?<![\\uD800-\\uDBFF])[\\uDC00-\\uDFFF]/;
    reply({ id: request.id, result: {
      text: request.params.text,
      path: request.params.path,
      wellFormed: !lone.test(request.params.text) && !lone.test(request.params.path),
    } });
    done();
  }`);

  const answer = await engine.engineRequest(
    compiler,
    "echo",
    { text: "const s = \"a\ud800b\";\n😀", path: "/w/\udc00.tt" },
    1000,
  );
  assert.deepEqual(answer, {
    result: {
      text: "const s = \"a�b\";\n😀",
      path: "/w/�.tt",
      wellFormed: true,
    },
  });
  engine.shutdownEngineServer();
});

test("a buffer with a lone surrogate is checked instead of ending the session", { skip, timeout: 10000 }, async () => {
  engine.retryEngineServer();
  engine.shutdownEngineServer();
  const answer = await engine.engineRequest(
    COMPILER,
    "check",
    { text: "const s = \"\ud800\";\nvariant S { A }\nconst v = match (S.A) { A => 1 };\n", filename: "a.tt", verify: true },
    5000,
  );
  assert.ok(answer && "result" in answer, `the compiler answered: ${JSON.stringify(answer)}`);
  const following = await check(COMPILER);
  assert.ok(following && "result" in following, "the session is still serving");
});
