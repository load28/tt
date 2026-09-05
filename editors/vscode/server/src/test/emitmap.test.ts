/* End-to-end tests for TypeScript answers over tt constructs, through the
 * engine session: hover inside match arms and pipelines, navigation out of
 * an arm body, imported unopened `.tt` modules, `@tt/std`, and type errors
 * mapped onto the source. The projection and every mapping live in the
 * engine now; what these lock is the observable result — the same results
 * the virtual-document pipeline (TASK-050/055/057/058/080) has always
 * produced, asked for and answered in `.tt` coordinates.
 *
 * They drive the real compiler *and* a real TypeScript language server, so
 * they skip when either is missing. */
import * as assert from "node:assert/strict";
import { after, test } from "node:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import * as engine from "../engine";
import { positionAt, sliceOf } from "./positions";
import { COMPILER, answered, compilerAvailable, findTsgo } from "./toolchain";
import { caseDir } from "./workspace";

const skip = !compilerAvailable()
  ? "no ttc — none built, installed, or on PATH"
  : findTsgo() === null
    ? "no tsgo executable"
    : false;

after(() => engine.shutdownEngineServer());

const SOURCE = [
  'import { disc } from "./helpers";',
  "",
  "variant Shape {",
  "  Circle(radius: number),",
  "  Rect(w: number, h: number),",
  "  Point,",
  "}",
  "",
  "declare function getShape(): Shape;",
  "const shape = getShape();",
  "",
  "const area = match (shape) {",
  "  Circle(radius) => disc(radius * radius),",
  "  Rect(w, h) => w * h,",
  "  Point => 0,",
  "};",
  "",
].join("\n");

/** A hand-written TypeScript file the source imports, so a definition that
 * leaves an arm body lands somewhere an editor can actually open. */
const HELPERS = "export function disc(x: number): number {\n  return x * Math.PI;\n}\n";

function fixture(name: string, files: Record<string, string>): string {
  const dir = caseDir(name);
  for (const [file, text] of Object.entries(files)) {
    fs.writeFileSync(path.join(dir, file), text);
  }
  return dir;
}

test(
  "hover inside a match arm body answers in source coordinates",
  { skip },
  async () => {
    // `radius` in the arm body is compiler-destructured in the emitted
    // switch — the raw text could never answer this; the engine's
    // projection does, and the span comes back in the arm body the user
    // wrote.
    const dir = fixture("tt-emitmap-test-", {
      "shapes.tt": SOURCE,
      "helpers.ts": HELPERS,
    });
    const file = path.join(dir, "shapes.tt");
    const info = await engine.hover(
      COMPILER,
      file,
      positionAt(SOURCE, SOURCE.indexOf("radius * radius")),
    );
    assert.ok(info, "expected quick info");
    assert.ok(
      info!.signature.includes("radius: number"),
      `signature was: ${info!.signature}`,
    );
    assert.equal(sliceOf(SOURCE, info!.range), "radius");
  },
);

test(
  "definition from an arm body lands in the hand-written file",
  { skip },
  async () => {
    const dir = fixture("tt-emitmap-test-", {
      "shapes.tt": SOURCE,
      "helpers.ts": HELPERS,
    });
    const file = path.join(dir, "shapes.tt");
    const defs = await engine.definition(
      COMPILER,
      file,
      positionAt(SOURCE, SOURCE.indexOf("disc(radius")),
    );
    assert.equal(defs.length, 1, JSON.stringify(defs));
    assert.equal(
      fs.realpathSync(defs[0].path),
      fs.realpathSync(path.join(dir, "helpers.ts")),
    );
    assert.equal(sliceOf(HELPERS, defs[0].range), "disc");
  },
);

/* --------------------------------------------------------------------------
 * TASK-055: the std module and imported `.tt` modules must reach the
 * language service with real types. Both used to arrive as `any` — the bare
 * `"@tt/std"` specifier resolved nowhere, and an imported `.tt` file was
 * served as raw source TypeScript can only error-recover through.
 * -------------------------------------------------------------------- */

const PIPE_SOURCE = [
  'import type { TResult } from "@tt/std";',
  'import * as Result from "@tt/std/result";',
  "",
  "declare function tokenize(s: string): TResult<string[], string>;",
  "declare function parse(t: string[]): TResult<number, string>;",
  "",
  "export function calculate(input: string): TResult<number, string> {",
  "  return input",
  "    |> .trim()",
  "    |> tokenize",
  "    |> Result.andThenP(parse);",
  "}",
  "",
].join("\n");

test("a pipeline step over the std module is not `any`", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": PIPE_SOURCE });
  const file = path.join(dir, "calc.tt");
  const info = await engine.hover(
    COMPILER,
    file,
    positionAt(PIPE_SOURCE, PIPE_SOURCE.indexOf("andThenP")),
  );
  assert.ok(info, "expected quick info");
  assert.ok(
    info!.signature.includes("TResult<number, string>"),
    `signature was: ${info!.signature}`,
  );
});

test(
  "a postfix pipeline step keeps its receiver's type",
  { skip },
  async () => {
    const dir = fixture("tt-std-test-", { "calc.tt": PIPE_SOURCE });
    const file = path.join(dir, "calc.tt");
    const info = await engine.hover(
      COMPILER,
      file,
      positionAt(PIPE_SOURCE, PIPE_SOURCE.indexOf("trim")),
    );
    assert.ok(info, "expected quick info");
    assert.ok(
      info!.signature.includes("String.trim(): string"),
      `signature was: ${info!.signature}`,
    );
  },
);

const IMPORTED_SHAPES = [
  "export variant Shape {",
  "  Circle(radius: number),",
  "  Rect(w: number, h: number),",
  "}",
  "",
].join("\n");

const IMPORTER = [
  'import { Shape } from "./shapes.tt";',
  "",
  "declare function getShape(): Shape;",
  "",
  "export const area = match (getShape()) {",
  "  Circle(radius) => Math.PI * radius * radius,",
  "  Rect(w, h) => w * h,",
  "};",
  "",
].join("\n");

test(
  "an imported .tt module is served emitted, not raw",
  { skip },
  async () => {
    // The importer is the open buffer; `shapes.tt` is only on disk. The
    // engine projects and serves it on its own — served raw, TypeScript
    // would read `variant Shape` as a TS variant and the arm bindings would lose
    // their types.
    const dir = fixture("tt-import-test-", {
      "shapes.tt": IMPORTED_SHAPES,
      "main.tt": IMPORTER,
    });
    const file = path.join(dir, "main.tt");
    engine.openDocument(COMPILER, file, IMPORTER);
    const info = await engine.hover(
      COMPILER,
      file,
      positionAt(IMPORTER, IMPORTER.indexOf("radius * radius")),
    );
    assert.ok(info, "expected quick info");
    assert.ok(
      info!.signature.includes("radius: number"),
      `signature was: ${info!.signature}`,
    );
    engine.closeDocument(COMPILER, file);
  },
);

/* --------------------------------------------------------------------------
 * TASK-057: type errors inside tt syntax reach the editor, every span on
 * the source the user wrote — a span that would land in generated code is
 * dropped, never reported at a made-up position.
 * -------------------------------------------------------------------- */

const BAD_PIPE = [
  'import type { TResult } from "@tt/std";',
  'import * as Result from "@tt/std/result";',
  "",
  "declare function evaluate(): TResult<number, string>;",
  "",
  "// `n` is a number; `n.length` is not a thing.",
  "export const bad = evaluate() |> Result.mapP((n) => n.length);",
  "",
].join("\n");

test("a type error inside a pipeline is reported", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": BAD_PIPE });
  const file = path.join(dir, "calc.tt");
  const diagnostics = answered(await engine.tsDiagnostics(COMPILER, file), "tsDiagnostics");
  assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics));
  assert.equal(diagnostics[0].code, 2339); // property does not exist
  assert.match(diagnostics[0].message, /'length' does not exist on type/);
  assert.equal(sliceOf(BAD_PIPE, diagnostics[0].range), "length");
});

const BAD_BOUNDARY = [
  "const inc = (n: number): number => n + 1;",
  "const shout = (s: string): string => s.toUpperCase();",
  "const a = 1 |> inc |> shout;",
  "",
].join("\n");

test("a pipeline boundary mismatch labels the producing step", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": BAD_BOUNDARY });
  const file = path.join(dir, "calc.tt");
  const diagnostics = answered(await engine.tsDiagnostics(COMPILER, file), "tsDiagnostics");
  assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics));
  assert.equal(sliceOf(BAD_BOUNDARY, diagnostics[0].range), "shout");
  const related = diagnostics[0].related ?? [];
  assert.equal(related.length, 1, JSON.stringify(diagnostics));
  assert.equal(related[0].message, "the piped value is produced here");
  assert.equal(sliceOf(BAD_BOUNDARY, related[0].range), "inc");
});

const BAD_ARM = [
  "variant Shape {",
  "  Circle(radius: number),",
  "  Point,",
  "}",
  "",
  "declare function getShape(): Shape;",
  "",
  "export const area = match (getShape()) {",
  "  Circle(radius) => radius.toUpperCase(),",
  "  Point => 0,",
  "};",
  "",
].join("\n");

test("a type error inside a match arm is reported", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": BAD_ARM });
  const file = path.join(dir, "calc.tt");
  const diagnostics = answered(await engine.tsDiagnostics(COMPILER, file), "tsDiagnostics");
  assert.equal(diagnostics.length, 1, JSON.stringify(diagnostics));
  assert.equal(diagnostics[0].code, 2339);
  assert.equal(sliceOf(BAD_ARM, diagnostics[0].range), "toUpperCase");
});

test("clean tt syntax reports nothing", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": PIPE_SOURCE });
  assert.deepEqual(
    answered(await engine.tsDiagnostics(COMPILER, path.join(dir, "calc.tt")), "tsDiagnostics"),
    [],
  );
});

test("a buffer mid-edit is never invented errors for", { skip }, async () => {
  // An unfinished construct does not lower — the projection passes the raw
  // text through — and TypeScript cannot parse tt syntax, so its recovery
  // would invent errors all over the file. The parse-error guard drops
  // every one of them: degrade to silence, never to lies.
  const broken = [
    "declare const shape: { kind: string };",
    "const label = match (shape) {",
    "  Circle(radius) => radius.",
  ].join("\n");
  const dir = fixture("tt-rawdiag-test-", { "shapes.tt": broken });
  const file = path.join(dir, "shapes.tt");
  engine.openDocument(COMPILER, file, broken);
  assert.deepEqual(answered(await engine.tsDiagnostics(COMPILER, file), "tsDiagnostics"), []);
  engine.closeDocument(COMPILER, file);
});

/* --------------------------------------------------------------------------
 * TASK-058: the type environment itself must be sound before anything is
 * reported — no invented TS2488 on a tuple destructuring the user wrote.
 * -------------------------------------------------------------------- */

const TUPLE_SOURCE = [
  'import type { TResult } from "@tt/std";',
  'import * as Result from "@tt/std/result";',
  "",
  "type Evaluated = TResult<number, string>;",
  "type Operands = TResult<[number, number], string>;",
  "",
  "export const applyP =",
  "  (f: (a: number, b: number) => number) =>",
  "  (ops: Operands): Evaluated =>",
  "    Result.map(ops, ([a, b]) => f(a, b));",
  "",
].join("\n");

test("tuple destructuring over the std module reports nothing", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": TUPLE_SOURCE });
  assert.deepEqual(
    answered(await engine.tsDiagnostics(COMPILER, path.join(dir, "calc.tt")), "tsDiagnostics"),
    [],
  );
});

/* --------------------------------------------------------------------------
 * TASK-080: the names a construct *introduces* — a `try` declaration, a
 * let-else / if let / match pattern binding — are copied from the source
 * into the emitted declaration, so hovering one answers with its type
 * instead of nothing.
 * -------------------------------------------------------------------- */

const BINDING_SOURCE = [
  'import type { TOption, TResult } from "@tt/std";',
  'import * as Option from "@tt/std/option";',
  'import * as Result from "@tt/std/result";',
  "",
  "declare function load(): TResult<number, string>;",
  "declare function boxed(): TOption<string>;",
  "",
  "export function run(): number {",
  "  const total = try load();",
  '  const Some(value: label) = boxed() else { throw new Error("none"); };',
  "  if let Some(value: shown) = boxed() {",
  "    console.log(shown);",
  "  }",
  "  const width = match (boxed()) {",
  "    Some(value: text) => text.length,",
  "    None => 0,",
  "  };",
  "  return total + label.length + width;",
  "}",
  "",
].join("\n");

/** Hovers the `needle` written inside `context` and returns the signature
 * TypeScript answers with. */
async function signatureOfBinding(
  file: string,
  context: string,
  needle: string,
): Promise<string> {
  const src = BINDING_SOURCE.indexOf(context) + context.indexOf(needle);
  const info = await engine.hover(
    COMPILER,
    file,
    positionAt(BINDING_SOURCE, src),
  );
  assert.ok(info, `expected quick info for ${needle}`);
  return info!.signature;
}

test("a try declaration's binding hovers with its Ok type", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": BINDING_SOURCE });
  const file = path.join(dir, "calc.tt");
  const signature = await signatureOfBinding(file, "total = try load()", "total");
  assert.match(signature, /total: number/);
});

test("let-else and if let bindings hover with the extracted type", { skip }, async () => {
  const dir = fixture("tt-std-test-", { "calc.tt": BINDING_SOURCE });
  const file = path.join(dir, "calc.tt");
  assert.match(
    await signatureOfBinding(file, "value: label)", "label"),
    /label: string/,
  );
  assert.match(
    await signatureOfBinding(file, "value: shown)", "shown"),
    /shown: string/,
  );
  assert.match(
    await signatureOfBinding(file, "value: text)", "text"),
    /text: string/,
  );
});

/* --------------------------------------------------------------------------
 * TASK-118: a diagnostic on generated code is restated in tt's words *and*
 * in tt's names — the case TypeScript printed structurally is called by the
 * declaration it lowers from, with TypeScript's own text alongside.
 * -------------------------------------------------------------------- */

const NAMED_SOURCE = [
  'import type { TResult } from "@tt/std";',
  'import * as Result from "@tt/std/result";',
  "",
  "variant Wire { OutOfRange(value: number), Missing }",
  "variant ParseError { NotANumber(text: string) }",
  "",
  "function inner(w: Wire) {",
  '  if (w.kind === "OutOfRange") { return Result.Err(w); }',
  "  return Result.Ok(1);",
  "}",
  "",
  "export function outer(w: Wire): TResult<number, ParseError> {",
  "  const n = try inner(w);",
  "  return Result.Ok(n);",
  "}",
  "",
].join("\n");

test("a restated diagnostic names the case it is about", { skip }, async () => {
  const dir = fixture("tt-named-test-", { "wire.tt": NAMED_SOURCE });
  const diagnostics = answered(await engine.tsDiagnostics(
    COMPILER,
    path.join(dir, "wire.tt"),
  ), "tsDiagnostics");
  const error = diagnostics.find((d) => d.code === 2322);
  assert.ok(error, JSON.stringify(diagnostics));
  // The propagation is the extent, and the wording is tt's (TASK-104/116).
  assert.equal(sliceOf(NAMED_SOURCE, error!.range), "try inner(w)");
  assert.match(error!.message, /the `Err` this `try` propagates/);
  // The narrowed case prints structurally; tt says whose case it is, and
  // a union covering a whole variant is that variant.
  assert.match(
    error!.message,
    /in tt's names: Type 'TErr<Wire\.OutOfRange>' is not assignable to type 'TResult<number, ParseError>'/,
  );
  // TypeScript's own text rides along, unchanged.
  assert.match(
    error!.message,
    /ts2322: Type 'TErr<\{ kind: "OutOfRange"; value: number; \}>'/,
  );
});

test(
  "a direct tt cause suppresses provisional checker consequences",
  { skip },
  async () => {
    const source = [
      "variant Conn { Up(value: number), Down }",
      "export const mixed = (c: Conn): string =>",
      '  match (c) { Up(value) => "up", 404 => "gone", Down => "down" };',
      "",
    ].join("\n");
    const dir = fixture("tt-owned-diagnostic-test-", { "mixed.tt": source });
    const diagnostics = answered(await engine.tsDiagnostics(
      COMPILER,
      path.join(dir, "mixed.tt"),
    ), "tsDiagnostics");
    assert.deepEqual(
      diagnostics,
      [],
      "the direct TT diagnostic owns every checker consequence of this lowering",
    );
  },
);
