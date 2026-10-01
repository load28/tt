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
import * as path from "node:path";

import * as engine from "../engine";
import { runCheck, runTypedCheck, unusableCompiler } from "../ttc";
import { COMPILER, compilerAvailable, findTsgo } from "./toolchain";
import { repoTestDir, testDir } from "../../../../../scripts/test-dirs.cjs";

after(() => engine.shutdownEngineServer());

const dir = testDir("tt-unusable-");

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

/* An engine that cannot answer is not an engine that answered "none".
 *
 * `tsDiagnostics` returned `[]` for both, so a session the engine could not
 * reach published a generation with no type errors at all and the Problems
 * panel went clean for a file that still had them. Nothing said so, and
 * nothing retried until the next keystroke (TASK-345). */
const typedSkip = !compilerAvailable()
  ? "no ttc — none built, installed, or on PATH"
  : findTsgo() === null
    ? "no tsgo executable"
    : false;

test("an unreachable engine answers null, not an empty diagnostics list", { skip: typedSkip, timeout: 60_000 }, async () => {
  const project = repoTestDir("tt-unreachable-");
  fs.writeFileSync(
    path.join(project, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: {
        strict: true,
        module: "preserve",
        moduleResolution: "bundler",
        noEmit: true,
      },
      include: ["*"],
    }),
  );
  const file = path.join(project, "main.tt");
  const source = 'export const bad: number = "text";\n';
  fs.writeFileSync(file, source);

  engine.openDocument(COMPILER, file, source);
  const answered = await engine.tsDiagnostics(COMPILER, file);
  assert.ok(
    answered?.some((d) => String(d.code) === "2322"),
    `the working engine reports the error: ${JSON.stringify(answered)}`,
  );

  // The same question, asked of a compiler that cannot serve.
  const unreachable = path.join(dir, "nowhere", "ttc");
  assert.equal(
    await engine.tsDiagnostics(unreachable, file),
    null,
    "no answer is null, so the caller cannot publish it as a clean file",
  );
});

test("one-shot checks own and remove their temporary input directories", { skip: process.platform === "win32" }, async () => {
  const project = repoTestDir("tt-check-lifetime-");
  const compiler = path.join(project, "compiler");
  const log = path.join(project, "inputs.jsonl");
  fs.writeFileSync(compiler, `#!/usr/bin/env node
const fs = require('node:fs');
const args = process.argv.slice(2);
if (args.includes('--server')) process.exit(1);
const input = args.at(-1);
fs.appendFileSync(${JSON.stringify(log)}, JSON.stringify(input) + '\\n');
const before = fs.readFileSync(input, 'utf8');
setTimeout(() => {
  if (fs.readFileSync(input, 'utf8') !== before) process.exit(2);
  process.exit(before.includes('crash') ? 2 : 0);
}, 100);
`);
  fs.chmodSync(compiler, 0o755);
  const results = await Promise.all([
    runCheck(compiler, "export const n = 1;", "same.tt", false),
    runCheck(compiler, "export const n = 2;", "same.tt", false),
  ]);
  assert.deepEqual(results.map(r => r.kind), ["ok", "ok"]);
  assert.equal((await runCheck(compiler, "crash", "same.tt", false)).kind, "failed");
  const inputs: string[] = fs.readFileSync(log, "utf8").trim().split("\n").map(line => JSON.parse(line));
  assert.equal(new Set(inputs).size, 3);
  for (const input of inputs) assert.equal(fs.existsSync(path.dirname(input)), false);
});

function oneShotCompiler(project: string): string {
  const compiler = path.join(project, "one-shot-ttc");
  fs.writeFileSync(compiler, `#!/usr/bin/env node
const { spawn } = require("node:child_process");
const args = process.argv.slice(2);
if (args.includes("--server")) process.exit(1);
const child = spawn(${JSON.stringify(COMPILER)}, args, { stdio: ["pipe", "inherit", "inherit"] });
child.stdin.on("error", () => {});
process.stdin.pipe(child.stdin);
child.on("exit", (code) => process.exit(code ?? 1));
`);
  fs.chmodSync(compiler, 0o755);
  return compiler;
}

test("the one-shot check reports UTF-16 columns after astral characters, as the engine does", { skip: compilerAvailable() ? process.platform === "win32" : "no ttc", timeout: 60_000 }, async () => {
  const project = repoTestDir("tt-oneshot-columns-");
  const compiler = oneShotCompiler(project);
  const line = 'const e = "\u{1F600}\u{1F600}"; const a = match (s) { Circel(radius) => radius, Empty => 0 };';
  const source = `variant Shape { Circle(radius: number), Empty }\ndeclare const s: Shape;\n${line}\n`;
  const column = (result: Awaited<ReturnType<typeof runCheck>>) =>
    result.kind === "ok" ? result.diagnostics.find((d) => d.code === "unknown-case")?.col : result.kind;

  assert.equal(column(await runCheck(COMPILER, source, "shape.tt", false)), line.indexOf("Circel") + 1);
  assert.equal(column(await runCheck(compiler, source, "shape.tt", false)), line.indexOf("Circel") + 1);
});

test("the one-shot typed check reports UTF-16 columns after astral characters, as the engine does", { skip: typedSkip || (process.platform === "win32" ? "posix wrapper" : false), timeout: 60_000 }, async () => {
  const project = repoTestDir("tt-oneshot-typed-columns-");
  const compiler = oneShotCompiler(project);
  const file = path.join(project, "main.tt");
  const source = 'const e = "\u{1F600}\u{1F600}"; export const value: number = "wrong";\n';
  fs.writeFileSync(file, source);
  const column = (result: Awaited<ReturnType<typeof runTypedCheck>>) =>
    result.kind === "ok" ? result.diagnostics.find((d) => d.code === "ts2322")?.col : JSON.stringify(result);

  const engineColumn = column(await runTypedCheck(COMPILER, source, file, true));
  assert.equal(engineColumn, source.indexOf("value:") + 1);
  assert.equal(column(await runTypedCheck(compiler, source, file, true)), engineColumn);
});
