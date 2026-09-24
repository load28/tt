/* Tests for on-save sidecar regeneration. These drive the real `ttc`
 * binary, which in turn drives a real TypeScript compiler (the declarations
 * are the compiler's; only the map is ttc's). They skip when either is
 * missing — the guard runs the same command the feature runs, so a skip
 * means "no toolchain", never "the refresh quietly did nothing". */
import * as assert from "node:assert/strict";
import { test } from "node:test";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import { WriteLedger, refreshSidecar, selfWrites } from "../sidecar";
import { COMPILER, compilerAvailable } from "./toolchain";
import { caseDir } from "./workspace";

/**
 * Whether a TypeScript that can *emit declarations* is resolvable — the
 * probe runs the same command the refresh runs, so a skip means the
 * toolchain cannot do this, never that the refresh quietly did nothing.
 * A TypeScript 7.0 can check but not emit; 7.1 added the API.
 */
function toolchainAvailable(): boolean {
  const dir = caseDir("tt-toolchain-probe-");
  try {
    fs.writeFileSync(path.join(dir, "probe.tt"), "export const n: number = 1;\n");
    // `-o` the way `refreshSidecar` passes it: on its own `--types` writes
    // into `.tt-types`, and the probe asks whether a sidecar can be built
    // where the editor puts one.
    execFileSync(COMPILER, ["--types", "probe.tt", "-o", "."], { cwd: dir, stdio: "pipe" });
    return fs.existsSync(path.join(dir, "probe.tt.d.ts"));
  } catch {
    return false;
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
}

const skip = !compilerAvailable()
  ? "no ttc — none built, installed, or on PATH"
  : !toolchainAvailable()
    ? "no TypeScript compiler for ttc to drive"
    : false;

const SOURCE = [
  "/** 알림 한 건. */",
  "export variant Notice {",
  "  Info(text: string),",
  "  Warn(text: string),",
  "}",
  "",
  "export function render(notice: Notice): string {",
  "  return match (notice) {",
  "    Info(text) => text,",
  "    Warn(text) => text.toUpperCase(),",
  "  };",
  "}",
  "",
].join("\n");

function workspace(): string {
  const dir = caseDir("tt-sidecar-test-");
  fs.writeFileSync(path.join(dir, "notice.tt"), SOURCE);
  return dir;
}

test("always mode writes both sidecar files", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  const result = await refreshSidecar(COMPILER, tt, "always");
  assert.equal(result.kind, "written", JSON.stringify(result));

  const declarations = fs.readFileSync(`${tt}.d.ts`, "utf8");
  const map = fs.readFileSync(`${tt}.d.ts.map`, "utf8");

  // The declarations describe what tt emitted: a union type and a
  // constructor object under the same name.
  assert.match(declarations, /export type Notice/);
  assert.match(declarations, /export declare const Notice/);
  assert.match(declarations, /export declare function render/);
  assert.match(declarations, /\/\/# sourceMappingURL=notice\.tt\.d\.ts\.map/);

  // The map points back at the .tt file — this is what sends "go to
  // definition" to the original instead of the .d.ts.
  const parsed = JSON.parse(map) as { sources: string[]; mappings: string };
  assert.deepEqual(parsed.sources, ["notice.tt"]);
  assert.ok(parsed.mappings.length > 0);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("ttx saves refresh TSX declarations and map back to the ttx source", { skip }, async () => {
  const dir = caseDir("ttx-sidecar-test-");
  const ttx = path.join(dir, "view.ttx");
  fs.writeFileSync(
    path.join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: {
        strict: true,
        declaration: true,
        jsx: "preserve",
      },
    }),
  );
  fs.writeFileSync(
    ttx,
    [
      "declare namespace JSX { interface IntrinsicElements { main: {} } }",
      "export variant State { Ready(label: string), Empty }",
      "export const View = ({ state }: { state: State }) => (",
      "  <main>{match (state) { Ready(label) => label, Empty => null }}</main>",
      ");",
      "",
    ].join("\n"),
  );

  const result = await refreshSidecar(COMPILER, ttx, "always");
  assert.equal(result.kind, "written", JSON.stringify(result));
  const declaration = fs.readFileSync(`${ttx}.d.ts`, "utf8");
  assert.match(declaration, /export type State/);
  assert.match(declaration, /export declare const View/);
  assert.match(declaration, /sourceMappingURL=view\.ttx\.d\.ts\.map/);
  const map = JSON.parse(fs.readFileSync(`${ttx}.d.ts.map`, "utf8")) as {
    sources: string[];
  };
  assert.deepEqual(map.sources, ["view.ttx"]);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("declarations can live in their own tree", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");
  const types = path.join(dir, ".tt-types");

  const result = await refreshSidecar(COMPILER, tt, "always", types);
  assert.equal(result.kind, "written", JSON.stringify(result));

  // The source tree stays clean; the declarations sit next to each other.
  assert.equal(fs.existsSync(`${tt}.d.ts`), false);
  assert.equal(fs.existsSync(path.join(types, "notice.tt.d.ts")), true);

  // `sources` has to cross the distance, or the map cannot find the source.
  const map = JSON.parse(
    fs.readFileSync(path.join(types, "notice.tt.d.ts.map"), "utf8"),
  ) as { sources: string[] };
  assert.deepEqual(map.sources, ["../notice.tt"]);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("refresh mode looks for the sidecar where it is written", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");
  const types = path.join(dir, ".tt-types");

  // Nothing there yet — refresh must not create it.
  assert.equal((await refreshSidecar(COMPILER, tt, "refresh", types)).kind, "skipped");

  await refreshSidecar(COMPILER, tt, "always", types);
  assert.equal((await refreshSidecar(COMPILER, tt, "refresh", types)).kind, "written");

  fs.rmSync(dir, { recursive: true, force: true });
});

test("refresh mode leaves a workspace that never opted in alone", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  const result = await refreshSidecar(COMPILER, tt, "refresh");
  assert.equal(result.kind, "skipped");
  assert.equal(fs.existsSync(`${tt}.d.ts`), false);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("refresh mode updates a sidecar that is already there", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  await refreshSidecar(COMPILER, tt, "always");
  // Anything exported after the first generation must show up on the next
  // save.
  fs.writeFileSync(tt, `${SOURCE}export function count(items: Notice[]): number {\n  return items.length;\n}\n`);

  const result = await refreshSidecar(COMPILER, tt, "refresh");
  assert.equal(result.kind, "written", JSON.stringify(result));
  assert.match(fs.readFileSync(`${tt}.d.ts`, "utf8"), /export declare function count/);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("a recoverable malformed node refreshes declarations around it", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  await refreshSidecar(COMPILER, tt, "always");
  // Parser-owned recovery replaces only the malformed pipeline node in the
  // editor projection. `--types` still exits with diagnostics, while the
  // independent declarations remain emit-capable and must stay fresh.
  fs.writeFileSync(tt, `${SOURCE}const broken = 1 |> ;\n`);
  const result = await refreshSidecar(COMPILER, tt, "refresh");

  assert.equal(result.kind, "written", JSON.stringify(result));
  assert.match(fs.readFileSync(`${tt}.d.ts`, "utf8"), /export type Notice/);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("a match that is not exhaustive still refreshes the sidecar", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  await refreshSidecar(COMPILER, tt, "always");

  // Exhaustiveness is a question about a type, so the checker answers it —
  // and it answers it *after* the declarations are emitted. The arm is
  // missing, the diagnostic says so, and the sidecar still describes the
  // new case, which is what the file being edited actually exports.
  fs.writeFileSync(tt, SOURCE.replace("  Warn(text: string),", "  Warn(text: string),\n  Debug(),"));
  const result = await refreshSidecar(COMPILER, tt, "refresh");

  assert.equal(result.kind, "written", JSON.stringify(result));
  assert.match(fs.readFileSync(`${tt}.d.ts`, "utf8"), /Debug/);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("off mode does nothing", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  const result = await refreshSidecar(COMPILER, tt, "off");
  assert.equal(result.kind, "skipped");
  assert.equal(fs.existsSync(`${tt}.d.ts`), false);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("a compiler terminated by a signal does not report sidecars as written", async () => {
  const dir = caseDir("tt-sidecar-signal-");
  try {
    const tt = path.join(dir, "source.tt");
    const compiler = path.join(dir, "terminated-compiler");
    fs.writeFileSync(tt, "export const value = 1;\n");
    fs.writeFileSync(compiler, "#!/bin/sh\nkill -TERM $$\n", { mode: 0o755 });
    const result = await refreshSidecar(compiler, tt, "always");
    assert.equal(result.kind, "failed", JSON.stringify(result));
    assert.equal(fs.existsSync(`${tt}.d.ts`), false);
  } finally {
    fs.rmSync(dir, { recursive: true, force: true });
  }
});

test("the write ledger owns a file while the disk holds what it wrote", () => {
  const dir = caseDir("tt-write-ledger-");
  const file = path.join(dir, "x.tt.d.ts");
  const ledger = new WriteLedger();

  const generation = ledger.expect([file]);
  assert.equal(ledger.owns(file), true, "a write in flight is its own");
  fs.writeFileSync(file, "export {};\n");
  ledger.settle(generation, true);
  assert.equal(ledger.owns(file), true, "what it wrote is its own");

  fs.writeFileSync(file, "export const edited = 1;\n");
  assert.equal(ledger.owns(file), false, "a hand edit is somebody else's");
  assert.equal(ledger.owns(file), false, "and stays so");
  assert.equal(ledger.owns(path.join(dir, "never.tt.d.ts")), false);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("a failed write and a superseded generation own nothing", () => {
  const dir = caseDir("tt-write-ledger-");
  const file = path.join(dir, "x.tt.d.ts");
  const ledger = new WriteLedger();

  const failed = ledger.expect([file]);
  ledger.settle(failed, false);
  assert.equal(ledger.owns(file), false, "nothing was written");

  const older = ledger.expect([file]);
  const newer = ledger.expect([file]);
  fs.writeFileSync(file, "export {};\n");
  ledger.settle(older, true);
  assert.equal(ledger.owns(file), true, "the newer write is still in flight");
  ledger.settle(newer, false);
  assert.equal(ledger.owns(file), false);

  fs.rmSync(dir, { recursive: true, force: true });
});

test("a refresh records its sidecar files as the server's own writes", { skip }, async () => {
  const dir = workspace();
  const tt = path.join(dir, "notice.tt");

  const result = await refreshSidecar(COMPILER, tt, "always");
  assert.equal(result.kind, "written", JSON.stringify(result));
  assert.equal(selfWrites.owns(`${tt}.d.ts`), true);
  assert.equal(selfWrites.owns(`${tt}.d.ts.map`), true);
  assert.equal(selfWrites.owns(tt), false, "the source is the user's");

  fs.appendFileSync(`${tt}.d.ts`, "export declare const byHand: number;\n");
  assert.equal(selfWrites.owns(`${tt}.d.ts`), false, "a hand-edited declaration is external");
  assert.equal(selfWrites.owns(`${tt}.d.ts.map`), true);

  fs.rmSync(dir, { recursive: true, force: true });
});
