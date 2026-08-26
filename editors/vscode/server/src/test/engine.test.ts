/* Tests for the engine session — the semantic API every editor feature now
 * consumes (engine.ts → `ttc --server` → the tt engine). The questions are
 * positions in `.tt` sources and the answers come back in `.tt`
 * coordinates; nothing here touches emitted TypeScript.
 *
 * They drive the real compiler and, behind it, a real TypeScript language
 * server, so they skip when either is missing — the guard mirrors the
 * engine's own resolution, so a skip means "no toolchain", never "the
 * feature quietly answered nothing". */
import * as assert from "node:assert/strict";
import { after, test } from "node:test";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";

import * as engine from "../engine";
import { positionAt, sliceOf, spanOf } from "./positions";
import { COMPILER, compilerAvailable, findTsgo } from "./toolchain";

const skip = !compilerAvailable()
  ? "ttc not on PATH"
  : findTsgo() === null
    ? "no tsgo executable"
    : false;

after(() => engine.shutdownEngineServer());

/** A workspace with a hand-written `.ts` and an `.tt` file. The `.tt`
 * content here is plain passthrough TypeScript — what these tests exercise
 * is the session itself: buffers, navigation across files, rename safety,
 * diagnostics. (The tt constructs get their own suites in emitmap/
 * completion tests.) */
const RENDER = [
  "// nothing here is a rename target",
  'import { describe } from "./user";',
  "export function render(state: \"idle\" | \"loading\"): string {",
  "  const label = describe(state);",
  "  const bad: number = label;",
  "  return label;",
  "}",
  "",
].join("\n");

function workspace(): { dir: string; tt: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "tt-engine-test-"));
  fs.mkdirSync(path.join(dir, "src"));
  fs.writeFileSync(
    path.join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: {
        strict: true,
        target: "es2022",
        module: "preserve",
        moduleResolution: "bundler",
        skipLibCheck: true,
        noEmit: true,
      },
      include: ["src"],
    }),
  );
  fs.writeFileSync(
    path.join(dir, "src/user.ts"),
    'export type State = "idle" | "loading";\nexport function describe(s: State): string {\n  return s;\n}\n',
  );
  const tt = path.join(dir, "src/render.tt");
  fs.writeFileSync(tt, RENDER);
  return { dir, tt };
}

const TTX_SOURCE = [
  'import { format } from "./format";',
  "declare global {",
  "  namespace JSX { interface IntrinsicElements { main: { children?: unknown } } }",
  "}",
  "export variant State { Ready(value: number), Empty }",
  "declare const state: State;",
  "export const View = () => {",
  "  const label = match (state) {",
  "    Ready(value) => format(value),",
  '    Empty => "empty",',
  "  };",
  "  const bad: number = label;",
  "  return <main>{label.toUpperCase()}</main>;",
  "};",
  "",
].join("\n");

function ttxWorkspace(): { dir: string; ttx: string } {
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), "ttx-engine-test-"));
  fs.mkdirSync(path.join(dir, "src"));
  fs.writeFileSync(
    path.join(dir, "tsconfig.json"),
    JSON.stringify({
      compilerOptions: {
        strict: true,
        target: "es2022",
        module: "preserve",
        moduleResolution: "bundler",
        jsx: "preserve",
        skipLibCheck: true,
        noEmit: true,
      },
      include: ["src"],
    }),
  );
  fs.writeFileSync(
    path.join(dir, "src/format.tsx"),
    "export function format(value: number): string { return value.toFixed(1); }\n",
  );
  const ttx = path.join(dir, "src/view.ttx");
  fs.writeFileSync(ttx, TTX_SOURCE);
  return { dir, ttx };
}

test("ttx receives the complete TypeScript and tt semantic surface", { skip }, async () => {
  const { dir, ttx } = ttxWorkspace();

  const diagnostics = await engine.tsDiagnostics(COMPILER, ttx);
  const mismatch = diagnostics.find((diagnostic) => diagnostic.code === 2322);
  assert.ok(mismatch, JSON.stringify(diagnostics));
  assert.equal(sliceOf(TTX_SOURCE, mismatch!.range), "bad");

  const hover = await engine.hover(
    COMPILER,
    ttx,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("label = match")),
  );
  assert.ok(hover, "hover has an answer in ttx");
  assert.match(hover!.signature, /label: string/);

  const completion = await engine.completion(
    COMPILER,
    ttx,
    positionAt(
      TTX_SOURCE,
      TTX_SOURCE.indexOf("label.toUpperCase") + "label.".length,
    ),
    true,
  );
  assert.ok(
    completion?.items.some((item) => item.label === "toUpperCase"),
    JSON.stringify(completion),
  );

  const definition = await engine.definition(
    COMPILER,
    ttx,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("format(value)")),
  );
  assert.equal(definition.length, 1, JSON.stringify(definition));
  assert.equal(
    fs.realpathSync(definition[0].path),
    fs.realpathSync(path.join(dir, "src/format.tsx")),
  );

  const references = await engine.references(
    COMPILER,
    ttx,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("label = match")),
  );
  assert.equal(references.length, 3, JSON.stringify(references));
  for (const reference of references) {
    assert.equal(fs.realpathSync(reference.path), fs.realpathSync(ttx));
    assert.equal(sliceOf(TTX_SOURCE, reference.range), "label");
  }

  const edits = await engine.rename(
    COMPILER,
    ttx,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("label = match")),
  );
  assert.equal(edits?.length, 3, JSON.stringify(edits));

  const signature = await engine.signatureHelp(
    COMPILER,
    ttx,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("format(value)") + "format(".length),
  );
  assert.ok(signature, "signature help has an answer in ttx");
  assert.match(signature!.signatures[0].label, /format/);

  const symbol = await engine.ttSymbol(
    COMPILER,
    ttx,
    TTX_SOURCE,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("Ready(value) =>")),
  );
  assert.equal(symbol?.name, "Ready");
  assert.equal(symbol?.variantName, "State");
  const ttCompletions = await engine.ttCompletions(
    COMPILER,
    ttx,
    TTX_SOURCE,
    positionAt(TTX_SOURCE, TTX_SOURCE.indexOf("Ready(value) =>")),
  );
  assert.ok(ttCompletions.some((item) => item.label === "Ready"));

  const tokens = await engine.semanticTokens(COMPILER, TTX_SOURCE, ttx);
  assert.ok(tokens?.some((token) => token.kind === "keyword"));
});

test("hover answers for a buffer the disk never saw", { skip }, async () => {
  const { tt } = workspace();
  // The disk copy never sees this text; the engine's overlay is the truth.
  const buffer = RENDER.replace("const label", "const tag = 1;\n  const label");
  engine.openDocument(COMPILER, tt, buffer);
  const info = await engine.hover(
    COMPILER,
    tt,
    positionAt(buffer, buffer.indexOf("tag = 1")),
  );
  assert.ok(info, "hover has an answer");
  assert.match(info!.signature, /tag: 1|tag: number/);
  engine.closeDocument(COMPILER, tt);
});

test("hover names the checker's view, at the source span", { skip }, async () => {
  const { tt } = workspace();
  const info = await engine.hover(
    COMPILER,
    tt,
    positionAt(RENDER, RENDER.indexOf("describe(state)")),
  );
  assert.ok(info, "hover has an answer");
  assert.match(info!.signature, /describe/);
  assert.match(info!.signature, /State/);
  assert.equal(sliceOf(RENDER, info!.range), "describe");
});

test("definition crosses into the hand-written file on disk", { skip }, async () => {
  const { dir, tt } = workspace();
  const locations = await engine.definition(
    COMPILER,
    tt,
    positionAt(RENDER, RENDER.indexOf("describe(state)")),
  );
  assert.equal(locations.length, 1, JSON.stringify(locations));
  assert.equal(
    fs.realpathSync(locations[0].path),
    fs.realpathSync(path.join(dir, "src/user.ts")),
  );
  // The span names the declaration, in the target file's own coordinates.
  const target = fs.readFileSync(locations[0].path, "utf8");
  assert.equal(sliceOf(target, locations[0].range), "describe");
});

test("diagnostics come back at positions in the .tt source", { skip }, async () => {
  const { tt } = workspace();
  const diagnostics = await engine.tsDiagnostics(COMPILER, tt);
  const error = diagnostics.find((d) => d.code === 2322);
  assert.ok(error, JSON.stringify(diagnostics));
  assert.equal(sliceOf(RENDER, error!.range), "bad");
});

test(
  "a malformed tt construct does not hide an independent try type error",
  { skip },
  async () => {
    const { tt } = workspace();
    const source = [
      "export function tryNonResult(value: number): number {",
      "  const n = try value;",
      "  return n;",
      "}",
      "const broken = 1 |> ;",
      "",
    ].join("\n");
    engine.openDocument(COMPILER, tt, source);
    const diagnostics = await engine.tsDiagnostics(COMPILER, tt);
    const error = diagnostics.find((d) => d.code === 2339);
    assert.ok(error, JSON.stringify(diagnostics));
    assert.equal(sliceOf(source, error!.range), "try value");
    engine.closeDocument(COMPILER, tt);
  },
);

test("references find every use, in source coordinates", { skip }, async () => {
  const { tt } = workspace();
  const references = await engine.references(
    COMPILER,
    tt,
    positionAt(RENDER, RENDER.indexOf("label = describe")),
  );
  assert.ok(references.length >= 2, JSON.stringify(references));
  for (const reference of references) {
    assert.equal(
      fs.realpathSync(reference.path),
      fs.realpathSync(tt),
      "all of them in the .tt file",
    );
    assert.equal(sliceOf(RENDER, reference.range), "label");
  }
});

test("rename names every place the name is written", { skip }, async () => {
  const { tt } = workspace();
  const edits = await engine.rename(
    COMPILER,
    tt,
    positionAt(RENDER, RENDER.indexOf("label = describe")),
  );
  assert.ok(edits, "the binding can be renamed");
  // Three uses of `label`; every span names the identifier, because the
  // engine refuses the rename whole if any edit cannot be mapped back.
  assert.equal(edits!.length, 3, JSON.stringify(edits));
  for (const edit of edits!) {
    assert.equal(fs.realpathSync(edit.path), fs.realpathSync(tt));
    assert.equal(sliceOf(RENDER, edit.range), "label");
  }
});

test(
  "renaming a destructuring shorthand keeps TypeScript's expansion",
  { skip },
  async () => {
    // What a tt pattern binding compiles to. TypeScript cannot rewrite
    // `{ value }` to the new name alone — it expands the shorthand — and
    // the engine carries that expansion in `newText`, or the rename would
    // silently rebind a different field.
    const { tt } = workspace();
    const source = [
      'import { describe } from "./user";',
      'export function render(o: { value: "idle" | "loading" }): string {',
      "  const { value } = o;",
      "  return describe(value);",
      "}",
      "",
    ].join("\n");
    engine.openDocument(COMPILER, tt, source);
    const edits = await engine.rename(
      COMPILER,
      tt,
      positionAt(source, source.indexOf("value } = o")),
    );
    assert.ok(edits, "the binding can be renamed");
    const declaration = edits!.find(
      (e) => spanOf(source, e.range).start === source.indexOf("value } = o"),
    );
    assert.ok(declaration, JSON.stringify(edits));
    assert.equal(declaration!.newText, "value: ttRenamePlaceholder");
    // An ordinary use is replaced by the bare name.
    const use = edits!.find(
      (e) => spanOf(source, e.range).start === source.indexOf("value)"),
    );
    assert.ok(use, JSON.stringify(edits));
    assert.ok(
      use!.newText === null || use!.newText === "ttRenamePlaceholder",
      JSON.stringify(use),
    );
    engine.closeDocument(COMPILER, tt);
  },
);

test("what cannot be renamed answers null rather than nothing", { skip }, async () => {
  const { tt } = workspace();
  // A comment is not a rename target; the engine says so, and answering
  // "no edits" instead would look like a rename that changed nothing.
  assert.equal(
    await engine.rename(
      COMPILER,
      tt,
      positionAt(RENDER, RENDER.indexOf("rename target")),
    ),
    null,
  );
});

test("signature help describes the call being written", { skip }, async () => {
  const { tt } = workspace();
  const help = await engine.signatureHelp(
    COMPILER,
    tt,
    positionAt(RENDER, RENDER.indexOf("describe(state)") + "describe(".length),
  );
  assert.ok(help, "the call site has help");
  assert.match(help!.signatures[0].label, /describe/);
  assert.equal(help!.signatures[0].parameters.length, 1);
});

test("an edit is answered against the new text", { skip }, async () => {
  const { tt } = workspace();
  engine.openDocument(COMPILER, tt, RENDER);
  assert.ok(
    (await engine.tsDiagnostics(COMPILER, tt)).some((d) => d.code === 2322),
  );
  engine.updateDocument(
    COMPILER,
    tt,
    RENDER.replace("  const bad: number = label;\n", ""),
  );
  assert.equal(
    (await engine.tsDiagnostics(COMPILER, tt)).filter((d) => d.code === 2322)
      .length,
    0,
    "the fix is seen without restarting anything",
  );
  engine.closeDocument(COMPILER, tt);
});

test("a closed document is the disk's again", { skip }, async () => {
  const { tt } = workspace();
  engine.openDocument(
    COMPILER,
    tt,
    RENDER.replace("  const bad: number = label;\n", ""),
  );
  assert.equal(
    (await engine.tsDiagnostics(COMPILER, tt)).filter((d) => d.code === 2322)
      .length,
    0,
  );
  engine.closeDocument(COMPILER, tt);
  // The disk copy still has the error; closing dropped the overlay.
  assert.ok(
    (await engine.tsDiagnostics(COMPILER, tt)).some((d) => d.code === 2322),
  );
});

/* Hints need no TypeScript at all — they are the parse-only surface, like
 * semantic tokens — so they answer wherever ttc itself is available. */
const skipTtOnly = compilerAvailable() ? false : "ttc not on PATH";

const DEAD_ARM = [
  "variant Shape {",
  "  Circle(radius: number),",
  "  Rect(width: number, height: number),",
  "}",
  "",
  "declare const shape: Shape;",
  "",
  "const area = match (shape) {",
  "  Circle(radius) => radius,",
  "  Rect(width) => width,",
  "  Circle(radius: r) => r,",
  "};",
  "",
].join("\n");

test("an unreachable arm comes back as a hint, not a diagnostic", { skip: skipTtOnly }, async () => {
  const { dir } = workspace();
  const file = path.join(dir, "src/dead.tt");
  fs.writeFileSync(file, DEAD_ARM);
  const hints = await engine.ttHints(COMPILER, file, DEAD_ARM);
  assert.equal(hints.length, 1, JSON.stringify(hints));
  assert.equal(hints[0].kind, "unreachableArm");
  assert.equal(sliceOf(DEAD_ARM, hints[0].range), "Circle(radius: r) => r");
});

test("a live match has no hints", { skip: skipTtOnly }, async () => {
  const { dir, tt } = workspace();
  assert.deepEqual(await engine.ttHints(COMPILER, tt, RENDER), []);
  const live = path.join(dir, "src/live.tt");
  const text = DEAD_ARM.replace("  Circle(radius: r) => r,\n", "");
  fs.writeFileSync(live, text);
  assert.deepEqual(await engine.ttHints(COMPILER, live, text), []);
});
