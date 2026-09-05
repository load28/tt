/* End-to-end tests of the language server itself, over LSP (TASK-062).
 *
 * The modules below it are unit-tested elsewhere; what only shows up here is
 * how the server *composes* them — which is where the reported bug lived:
 * `Result.` answered with the variant's two constructors instead of, rather
 * than alongside, everything the TypeScript language service knows about
 * the standard library namespace.
 *
 * The server is spawned as the editor spawns it (`--stdio`) and driven with
 * a minimal JSON-RPC client, so the assertions are on what an editor would
 * actually receive. It runs the real compiler, so these skip when it is not
 * on PATH.
 */
import * as assert from "node:assert/strict";
import { test } from "node:test";
import { execFileSync, spawn, ChildProcess } from "node:child_process";
import * as fs from "node:fs";
import * as os from "node:os";
import * as path from "node:path";
import { pathToFileURL } from "node:url";

import { COMPILER, compilerAvailable, findTsgo } from "./toolchain";
import { caseDir } from "./workspace";

const SERVER = path.join(__dirname, "..", "server.js");
const skip = compilerAvailable() ? false : "no ttc — none built, installed, or on PATH";
/** Answers that need the TypeScript language service. A skip must mean a
 * tool is missing, never that a feature quietly answered nothing — so the
 * cases below that ask TypeScript guard on tsgo as well as on ttc. */
const skipTyped = skip || (findTsgo() ? false : "tsgo not installed");
/** Each case spawns a server and compiles through it; generous, and only
 * reached when something has hung. */
const timeout = 60_000;

for (const consumerKind of ["tt", "ttx"]) {
  for (const providerKind of ["tt", "ttx", "ts", "tsx"]) {
    test(`filesystem and config changes refresh ${providerKind} -> ${consumerKind}`, { skip: skipTyped, timeout }, async () => {
      const dir = caseDir("tt-filesystem-edit-");
      const provider = path.join(dir, `provider.${providerKind}`);
      const consumer = path.join(dir, `consumer.${consumerKind}`);
      const configPath = path.join(dir, "tsconfig.json");
      const config = { compilerOptions: { strict: true, noImplicitAny: false, module: "preserve", moduleResolution: "bundler", jsx: "preserve", noEmit: true, allowImportingTsExtensions: true }, include: ["*"] };
      fs.writeFileSync(configPath, JSON.stringify(config));
      fs.writeFileSync(consumer, "export {};\n");
      const source = `import { value } from "./provider.${providerKind}";\nconst result: string = value;\nexport function identity(input) { return input; }\n`;
      const uri = pathToFileURL(consumer).toString();
      const client = connect();
      const expect = (code?: string) => client.waitFor("textDocument/publishDiagnostics", p => p.uri === uri && (code ? p.diagnostics.some((d: any) => String(d.code) === code) : p.diagnostics.length === 0));
      const changed = (file: string, type: number) => client.notify("workspace/didChangeWatchedFiles", { changes: [{ uri: pathToFileURL(file).toString(), type }] });
      try {
        await client.request("initialize", { processId: process.pid, rootUri: pathToFileURL(dir).toString(), workspaceFolders: [{ uri: pathToFileURL(dir).toString(), name: "test" }], capabilities: {} });
        client.notify("initialized", {});
        let answer = expect("ts2307");
        client.notify("textDocument/didOpen", { textDocument: { uri, languageId: consumerKind, version: 1, text: source } });
        await answer;
        answer = expect();
        fs.writeFileSync(provider, 'export const value: string = "created";\n');
        changed(provider, 1);
        await answer;
        answer = expect("ts2322");
        fs.writeFileSync(provider, 'export const value: number = 42;\n');
        changed(provider, 2);
        await answer;
        answer = expect("ts2307");
        fs.unlinkSync(provider);
        changed(provider, 3);
        await answer;
        answer = expect();
        fs.writeFileSync(provider, 'export const value: string = "restored";\n');
        changed(provider, 1);
        await answer;
        answer = expect("ts7006");
        config.compilerOptions.noImplicitAny = true;
        fs.writeFileSync(configPath, JSON.stringify(config));
        changed(configPath, 2);
        assert.equal((await answer).version, 1, "the unsaved function survived project reload");
        answer = expect();
        config.compilerOptions.noImplicitAny = false;
        fs.writeFileSync(configPath, JSON.stringify(config));
        changed(configPath, 2);
        await answer;
        assert.equal(fs.readFileSync(consumer, "utf8"), "export {};\n", "reload never saves the buffer");
      } finally { client.stop(); }
    });

    test(`unsaved ${providerKind} changes refresh untouched ${consumerKind} diagnostics`, { skip: skipTyped, timeout }, async () => {
      const dir = caseDir("tt-dependency-edit-");
      const provider = path.join(dir, `provider.${providerKind}`);
      const consumer = path.join(dir, `consumer.${consumerKind}`);
      const original = 'export const value: string = "disk";\n';
      const source = `import { value } from "./provider.${providerKind}";\nconst result: string = value;\n`;
      fs.writeFileSync(provider, original);
      fs.writeFileSync(consumer, source);
      fs.writeFileSync(path.join(dir, "tsconfig.json"), JSON.stringify({
        compilerOptions: { strict: true, module: "preserve", moduleResolution: "bundler", jsx: "preserve", noEmit: true, allowImportingTsExtensions: true },
        include: ["*"],
      }));
      const uri = pathToFileURL(consumer).toString();
      const providerUri = pathToFileURL(provider).toString();
      const client = connect();
      try {
        await client.request("initialize", {
          processId: process.pid, rootUri: pathToFileURL(dir).toString(),
          workspaceFolders: [{ uri: pathToFileURL(dir).toString(), name: "test" }], capabilities: {},
        });
        client.notify("initialized", {});
        // Open the host first: the project must exist before any tt request.
        client.notify("textDocument/didOpen", { textDocument: {
          uri: providerUri, languageId: providerKind === "ts" ? "typescript" : providerKind === "tsx" ? "typescriptreact" : providerKind,
          version: 1, text: original,
        } });
        const clean = client.waitFor("textDocument/publishDiagnostics", p => p.uri === uri && p.diagnostics.length === 0);
        client.notify("textDocument/didOpen", { textDocument: { uri, languageId: consumerKind, version: 1, text: source } });
        await clean;
        const failed = client.waitFor("textDocument/publishDiagnostics", p => p.uri === uri && p.diagnostics.some((d: any) => String(d.code) === "ts2322"));
        client.notify("textDocument/didChange", {
          textDocument: { uri: providerUri, version: 2 }, contentChanges: [{ text: "export const value: number = 42;\n" }],
        });
        assert.equal((await failed).version, 1, "consumer was never edited");
        const cleared = client.waitFor("textDocument/publishDiagnostics", p => p.uri === uri && p.diagnostics.length === 0);
        client.notify("textDocument/didClose", { textDocument: { uri: providerUri } });
        assert.equal((await cleared).version, 1, "closing reveals the disk dependency");
      } finally { client.stop(); }
    });
  }
}

/* A window's folders are not what it started with: people add and remove
 * them all day. Every folder is a place the compiler, the TypeScript
 * toolchain and a relative `tt.sidecarDir` are resolved from, and the
 * client sends the change notification only to a server that declared it
 * wants one — so without the capability the roots stayed frozen at
 * startup, for the life of the session (TASK-342). */
test("the server asks for folder changes, and acts on them", { skip, timeout }, async () => {
  const dir = caseDir("tt-folders-");
  const added = caseDir("tt-folders-added-");
  const file = path.join(dir, "main.tt");
  const source = "variant State { Ready, Empty }\ndeclare const state: State;\nexport const label = match (state) { Ready => \"r\" };\n";
  fs.writeFileSync(file, source);
  const uri = pathToFileURL(file).toString();
  const client = connect();
  try {
    const init = await client.request("initialize", {
      processId: process.pid,
      rootUri: pathToFileURL(dir).toString(),
      workspaceFolders: [{ uri: pathToFileURL(dir).toString(), name: "first" }],
      // What VS Code declares, and the only capability this case needs:
      // still no `workspace.configuration`, which this client would not
      // answer.
      capabilities: { workspace: { workspaceFolders: true } },
    });
    const folders = init.result.capabilities.workspace?.workspaceFolders;
    assert.equal(folders?.supported, true, JSON.stringify(init.result.capabilities.workspace));
    assert.ok(folders?.changeNotifications, "the client registers its listener on this alone");
    client.notify("initialized", {});

    const opened = client.waitFor("textDocument/publishDiagnostics", p => p.uri === uri && p.diagnostics.some((d: any) => String(d.code) === "match-not-exhaustive"));
    client.notify("textDocument/didOpen", { textDocument: { uri, languageId: "tt", version: 1, text: source } });
    await opened;
    // Let the generation that answered settle, so the publish awaited below
    // can only be the one the notification causes.
    await new Promise(resolve => setTimeout(resolve, 1000));

    const revalidated = client.waitFor("textDocument/publishDiagnostics", p => p.uri === uri);
    client.notify("workspace/didChangeWorkspaceFolders", {
      event: { added: [{ uri: pathToFileURL(added).toString(), name: "second" }], removed: [] },
    });
    assert.ok((await revalidated).diagnostics.some((d: any) => String(d.code) === "match-not-exhaustive"), "the open buffer is re-validated against the new roots");
  } finally { client.stop(); }
});

interface Client {
  request(method: string, params: unknown): Promise<any>;
  notify(method: string, params: unknown): void;
  /** The next notification of `method` that satisfies `want` — the server
   * publishes diagnostics on its own schedule, so a test waits for one. */
  waitFor(method: string, want: (params: any) => boolean): Promise<any>;
  stop(): void;
}

/** The framing an LSP client speaks: `Content-Length` headers over stdio. */
function connect(): Client {
  const child: ChildProcess = spawn(process.execPath, [SERVER, "--stdio"], {
    stdio: ["pipe", "pipe", "ignore"],
    // The LSP case lives in a temporary project, while the test contract is
    // against the compiler built from this checkout. Cover both supported
    // development routes: a linked package consumes TTC_BINARY, and the
    // final PATH fallback finds the same executable when no package exists.
    env: {
      ...process.env,
      TTC_BINARY: COMPILER,
      PATH: `${path.dirname(COMPILER)}${path.delimiter}${process.env.PATH ?? ""}`,
    },
  });
  const pending = new Map<number, (body: any) => void>();
  interface Waiter {
    method: string;
    want: (params: any) => boolean;
    resolve: (params: any) => void;
  }
  const waiters = new Map<number, Waiter>();
  let nextWaiter = 1;
  let nextId = 1;
  let buf = Buffer.alloc(0);

  child.stdout!.on("data", (chunk: Buffer) => {
    buf = Buffer.concat([buf, chunk]);
    for (;;) {
      const sep = buf.indexOf("\r\n\r\n");
      if (sep < 0) return;
      const length = /content-length: (\d+)/i.exec(
        buf.subarray(0, sep).toString(),
      );
      if (!length) return;
      const size = Number(length[1]);
      if (buf.length < sep + 4 + size) return;
      const body = JSON.parse(buf.subarray(sep + 4, sep + 4 + size).toString());
      buf = buf.subarray(sep + 4 + size);
      const resolve = body.id !== undefined ? pending.get(body.id) : undefined;
      if (resolve) {
        pending.delete(body.id);
        resolve(body);
        continue;
      }
      if (body.method !== undefined) {
        for (const [n, waiter] of [...waiters.entries()]) {
          if (waiter.method === body.method && waiter.want(body.params)) {
            waiters.delete(n);
            waiter.resolve(body.params);
          }
        }
      }
    }
  });

  const send = (message: unknown): void => {
    const text = JSON.stringify({ jsonrpc: "2.0", ...(message as object) });
    child.stdin!.write(
      `Content-Length: ${Buffer.byteLength(text)}\r\n\r\n${text}`,
    );
  };
  return {
    request: (method, params) =>
      new Promise((resolve) => {
        const id = nextId++;
        pending.set(id, resolve);
        send({ id, method, params });
      }),
    notify: (method, params) => send({ method, params }),
    waitFor: (method, want) =>
      new Promise((resolve) => {
        waiters.set(nextWaiter++, { method, want, resolve });
      }),
    stop: () => child.kill(),
  };
}

/** A server with `source` open as a tt-family document, ready to be asked. */
async function open(source: string, languageId: "tt" | "ttx" = "tt") {
  const dir = caseDir("tt-server-test-");
  const file = path.join(dir, `main.${languageId}`);
  fs.writeFileSync(file, source);
  const uri = pathToFileURL(file).toString();
  const client = connect();
  await client.request("initialize", {
    processId: process.pid,
    rootUri: pathToFileURL(dir).toString(),
    workspaceFolders: [{ uri: pathToFileURL(dir).toString(), name: "test" }],
    // No `workspace.configuration`: the server then uses its defaults
    // instead of asking, which this client would not answer.
    capabilities: {},
  });
  client.notify("initialized", {});
  client.notify("textDocument/didOpen", {
    textDocument: { uri, languageId, version: 1, text: source },
  });

  /** Completion at the end of `marker`'s first occurrence. */
  const completion = async (marker: string) => {
    const offset = source.indexOf(marker) + marker.length;
    assert.notEqual(offset, marker.length - 1, `marker not found: ${marker}`);
    const before = source.slice(0, offset);
    const line = before.split("\n").length - 1;
    const response = await client.request("textDocument/completion", {
      textDocument: { uri },
      position: {
        line,
        character: before.length - (before.lastIndexOf("\n") + 1),
      },
      context: { triggerKind: 2, triggerCharacter: "." },
    });
    const items = (
      Array.isArray(response.result)
        ? response.result
        : (response.result?.items ?? [])
    ) as any[];
    return {
      items,
      labels: items.map((i) => i.label as string),
      /** The resolved form of one item, as the editor asks for it when the
       * user highlights it. */
      resolve: async (label: string) => {
        const item = items.find((i) => i.label === label);
        assert.ok(item, `no completion item ${label} in: ${items.map((i) => i.label)}`);
        return (await client.request("completionItem/resolve", item)).result;
      },
    };
  };

  return { client, uri, completion, stop: () => client.stop() };
}

const TTX_EDITOR_SOURCE = [
  "declare global {",
  "  namespace JSX { interface IntrinsicElements { main: { children?: unknown } } }",
  "}",
  "variant State { Ready(value: string), Empty }",
  "declare const state: State;",
  "export const View = () => {",
  "  const label = match (state) {",
  "    Ready(value) => value,",
  '    Empty => "empty",',
  "  };",
  "  const bad: number = label;",
  "  return <main>{label.toUpperCase()}</main>;",
  "};",
  "",
].join("\n");

test(
  "ttx documents receive completion, hover, diagnostics, and semantic tokens",
  { skip: skipTyped, timeout },
  async () => {
    const { client, uri, completion, stop } = await open(TTX_EDITOR_SOURCE, "ttx");
    try {
      const diagnosticPromise = client.waitFor(
        "textDocument/publishDiagnostics",
        (params) =>
          params.uri === uri &&
          params.diagnostics.some((diagnostic: any) => diagnostic.code === "ts2322"),
      );
      const members = await completion("label.");
      assert.ok(
        members.labels.includes("toUpperCase"),
        `missing toUpperCase in: ${members.labels}`,
      );

      const hover = await client.request("textDocument/hover", {
        textDocument: { uri },
        position: positionOf(TTX_EDITOR_SOURCE, "const lab"),
      });
      assert.match(String(hover.result?.contents?.value ?? ""), /label: string/);

      const published = await diagnosticPromise;
      const mismatch = published.diagnostics.find(
        (diagnostic: any) => diagnostic.code === "ts2322",
      );
      assert.equal(covered(TTX_EDITOR_SOURCE, mismatch.range), "label");

      const semantic = await client.request("textDocument/semanticTokens/full", {
        textDocument: { uri },
      });
      assert.ok(semantic.result?.data?.length > 0, JSON.stringify(semantic.result));
    } finally {
      stop();
    }
  },
);

const STD_SOURCE = [
  'import type { TOption, TResult } from "@tt/std";',
  'import * as Option from "@tt/std/option";',
  'import * as Result from "@tt/std/result";',
  "",
  "declare const r: TResult<number, string>;",
  "const out = Result.",
  "",
].join("\n");

test(
  "`Result.` completes the constructors and the combinators",
  { skip: skipTyped, timeout },
  async () => {
    const { completion, stop } = await open(STD_SOURCE);
    try {
      const { labels, resolve } = await completion("const out = Result.");
      // tt's own: the case constructors, first in the list.
      assert.deepEqual(labels.slice(0, 2), ["Ok", "Err"]);
      // TypeScript's: the standard library combinators that used to be
      // dropped on the floor by returning only the constructors.
      for (const member of ["map", "andThen", "unwrapOr", "mapErrP", "ok"]) {
        assert.ok(labels.includes(member), `missing ${member} in: ${labels}`);
      }

      const resolved = await resolve("map");
      assert.ok(
        String(resolved.detail).includes("TResult<U, E>"),
        `detail was: ${resolved.detail}`,
      );
      assert.ok(
        String(resolved.documentation?.value).includes("Applies `f`"),
        `documentation was: ${JSON.stringify(resolved.documentation)}`,
      );
    } finally {
      stop();
    }
  },
);

const PIPE_SOURCE = ["const n: number = 1;", "const s = n", "  |> .", ""].join(
  "\n",
);

test(
  "a pipeline method step completes the piped value's members",
  { skip: skipTyped, timeout },
  async () => {
    const { completion, stop } = await open(PIPE_SOURCE);
    try {
      const { labels, resolve } = await completion("  |> .");
      for (const member of ["toFixed", "toString", "toPrecision"]) {
        assert.ok(labels.includes(member), `missing ${member} in: ${labels}`);
      }
      // A member access is members only — no variant names, no tt snippets.
      for (const noise of ["Option", "Result", "match", "variant"]) {
        assert.ok(!labels.includes(noise), `unexpected ${noise} in: ${labels}`);
      }
      const resolved = await resolve("toFixed");
      assert.ok(
        String(resolved.detail).includes("fractionDigits"),
        `detail was: ${resolved.detail}`,
      );
    } finally {
      stop();
    }
  },
);

test(
  "a member access in a step never falls back to the global scope",
  { skip: skipTyped, timeout },
  async () => {
    // Recovering from `|>`, TypeScript can lose the dot and answer with
    // every name in scope — the compiler's own `$tt_ap` helper included.
    // That answer is not a member list and must not be shown as one.
    const source = ["const n: number = 1;", "const s = n", "  |> n.", ""].join(
      "\n",
    );
    const { completion, stop } = await open(source);
    try {
      const { labels } = await completion("  |> n.");
      assert.ok(labels.includes("toFixed"), `members were: ${labels}`);
      for (const leaked of ["$tt_ap", "n", "s", "AbortController"]) {
        assert.ok(
          !labels.includes(leaked),
          `global scope leaked ${leaked} into: ${labels}`,
        );
      }
    } finally {
      stop();
    }
  },
);

test(
  "a pipeline of std combinators keeps completing at each step",
  { skip: skipTyped, timeout },
  async () => {
    const source = [
      'import type { TResult } from "@tt/std";',
      'import * as Result from "@tt/std/result";',
      "",
      "declare const r: TResult<number, string>;",
      "const out = r",
      "  |> Result.mapP((n) => n + 1)",
      "  |> Result.",
      "",
    ].join("\n");
    const { completion, stop } = await open(source);
    try {
      const { labels } = await completion("  |> Result.");
      for (const member of ["Ok", "Err", "mapP", "andThenP", "unwrapOrP"]) {
        assert.ok(labels.includes(member), `missing ${member} in: ${labels}`);
      }
    } finally {
      stop();
    }
  },
);

/* ---------------------------------------------------------------- semantic
 * tokens — the parser's classification, layered over the grammar (TASK-093).
 * Parse-only on the engine side, so unlike the completion cases these need
 * only the compiler, never a TypeScript toolchain.
 * -------------------------------------------------------------------------- */

/** The legend the server declares — mirrored here to decode the response. */
const TOKEN_TYPES = [
  "keyword",
  "enum",
  "enumMember",
  "variable",
  "property",
  "function",
  "operator",
];

/** Decodes the LSP delta-encoded quintuples into absolute tokens. */
function decodeTokens(
  data: number[],
): { line: number; character: number; length: number; type: string }[] {
  const out = [];
  let line = 0;
  let character = 0;
  for (let i = 0; i < data.length; i += 5) {
    line += data[i];
    character = data[i] === 0 ? character + data[i + 1] : data[i + 1];
    out.push({
      line,
      character,
      length: data[i + 2],
      type: TOKEN_TYPES[data[i + 3]],
    });
  }
  return out;
}

test(
  "semantic tokens carry the parser's own classification",
  { skip, timeout },
  async () => {
    const source = [
      "declare function match(n: number): number;",
      "const denied = match(1);",
      "const composed = flow",
      "  |> trim",
      "  |> parse;",
      "export function pick(shape: Shape): number {",
      "  return match (shape) {",
      "    Circle(r) => r,",
      "    _ => 0,",
      "  };",
      "}",
      "variant Shape { Circle(r: number), Point }",
      "",
    ].join("\n");
    const { client, uri, stop } = await open(source);
    try {
      const response = await client.request("textDocument/semanticTokens/full", {
        textDocument: { uri },
      });
      const tokens = decodeTokens(response.result?.data ?? []);
      const at = (line: number, character: number) =>
        tokens.find((t) => t.line === line && t.character === character);

      // The call to a plain function named `match` is *denied* — reported
      // as the function it is, overriding the grammar's keyword color.
      assert.equal(at(1, 15)?.type, "function");
      // A `flow` head whose first `|>` sits on the next line is *claimed* —
      // the grammar's same-line lookahead cannot see it, the parser can.
      assert.equal(at(2, 17)?.type, "keyword");
      // The real match expression and its pattern.
      assert.equal(at(6, 9)?.type, "keyword");
      assert.equal(at(7, 4)?.type, "enumMember");
      assert.equal(at(7, 11)?.type, "variable");
      // tt's Variant concept stays on the standard LSP `enum` wire token.
      assert.equal(at(11, 8)?.type, "enum");
    } finally {
      stop();
    }
  },
);

/* ---------------------------------------------------------------------- *
 * tt's own names (TASK-107): hover, definition and completion for the
 * three name spaces that exist only in `.tt` source. The engine answers
 * these from the compiler's declaration table, so — unlike everything
 * above — they need no TypeScript toolchain at all.
 * ---------------------------------------------------------------------- */

const SHAPE_SOURCE = [
  "variant Shape { Circle(radius: number), Rect(w: number, h: number), Point }",
  "declare const s: Shape;",
  "const area = match (s) {",
  "  Circle(radius) => radius,",
  "  Rect(w, h) => w * h,",
  "  Point => 0,",
  "};",
  "if let Circle(radius: r) = s { use(r); }",
  "",
].join("\n");

/** The position just past `marker`'s first occurrence, as LSP counts. */
function positionOf(source: string, marker: string) {
  const offset = source.indexOf(marker) + marker.length;
  const before = source.slice(0, offset);
  return {
    line: before.split("\n").length - 1,
    character: before.length - (before.lastIndexOf("\n") + 1),
  };
}

test("a case tag hovers as its declaration — in a match and in an if let", { skip, timeout }, async () => {
  const { client, uri, stop } = await open(SHAPE_SOURCE);
  try {
    for (const marker of ["  Circ", "if let Circ"]) {
      const answer = await client.request("textDocument/hover", {
        textDocument: { uri },
        position: positionOf(SHAPE_SOURCE, marker),
      });
      const value = String(answer.result?.contents?.value ?? "");
      assert.ok(
        value.includes("Shape.Circle(radius: number)"),
        `hover at ${marker} was: ${value}`,
      );
    }
  } finally {
    stop();
  }
});

test("a payload field hovers as its declaration", { skip, timeout }, async () => {
  const { client, uri, stop } = await open(SHAPE_SOURCE);
  try {
    const answer = await client.request("textDocument/hover", {
      textDocument: { uri },
      position: positionOf(SHAPE_SOURCE, "  Circle(rad"),
    });
    const value = String(answer.result?.contents?.value ?? "");
    assert.ok(value.includes("radius: number"), `hover was: ${value}`);
  } finally {
    stop();
  }
});

test("a pattern tag goes to its declaration", { skip, timeout }, async () => {
  const { client, uri, stop } = await open(SHAPE_SOURCE);
  try {
    const answer = await client.request("textDocument/definition", {
      textDocument: { uri },
      position: positionOf(SHAPE_SOURCE, "  Circ"),
    });
    const location = Array.isArray(answer.result)
      ? answer.result[0]
      : answer.result;
    assert.equal(location?.range?.start?.line, 0, "the declaration is line 0");
    assert.equal(location?.uri, uri);
  } finally {
    stop();
  }
});

test("pattern positions complete cases and fields", { skip, timeout }, async () => {
  const { completion, stop } = await open(SHAPE_SOURCE);
  try {
    // An arm position: every case, with the wildcard.
    const arm = await completion("  Rect(w, h) => w * h,\n  ");
    for (const label of ["Circle", "Rect", "Point", "_"]) {
      assert.ok(arm.labels.includes(label), `missing ${label} in: ${arm.labels}`);
    }
    // A payload position: that case's fields, and nothing else.
    const payload = await completion("  Rect(");
    assert.deepEqual(payload.labels, ["w", "h"]);
    // An `if let` — a position this server could not complete at all before.
    const conditional = await completion("if let ");
    assert.ok(
      conditional.labels.includes("Circle"),
      `missing Circle in: ${conditional.labels}`,
    );
  } finally {
    stop();
  }
});

test("references, rename, signature help, and document symbols cross the LSP adapter", { skip: skipTyped, timeout }, async () => {
  const source = [
    "variant Shape { Circle(radius: number), Point }",
    "function format(value: string, width?: number): string {",
    '  return value.padStart(width ?? 0, " ");',
    "}",
    'const label = "tt";',
    "export const output = format(label, 4);",
    "void label;",
    "",
  ].join("\n");
  const { client, uri, stop } = await open(source);
  try {
    const references = await client.request("textDocument/references", {
      textDocument: { uri },
      position: positionOf(source, "const lab"),
      context: { includeDeclaration: true },
    });
    assert.equal(references.result.length, 3, JSON.stringify(references.result));
    assert.ok(
      references.result.every(
        (location: any) =>
          location.uri === uri && covered(source, location.range) === "label",
      ),
      JSON.stringify(references.result),
    );

    const rename = await client.request("textDocument/rename", {
      textDocument: { uri },
      position: positionOf(source, "const lab"),
      newName: "title",
    });
    assert.equal(rename.result.changes[uri].length, 3);
    assert.ok(
      rename.result.changes[uri].every((edit: any) => edit.newText === "title"),
      JSON.stringify(rename.result),
    );

    const signature = await client.request("textDocument/signatureHelp", {
      textDocument: { uri },
      position: positionOf(source, "output = format("),
      context: { triggerKind: 1, isRetrigger: false },
    });
    assert.match(signature.result.signatures[0].label, /format/);
    assert.equal(signature.result.activeParameter, 0);

    const symbols = await client.request("textDocument/documentSymbol", {
      textDocument: { uri },
    });
    const shape = symbols.result.find((symbol: any) => symbol.name === "Shape");
    assert.ok(shape, JSON.stringify(symbols.result));
    assert.deepEqual(
      shape.children.map((symbol: any) => symbol.name),
      ["Circle", "Point"],
    );
  } finally {
    stop();
  }
});

/* ------------------------------------------------------------------ */
/* diagnostic ranges (TASK-116)                                        */
/* ------------------------------------------------------------------ */

/** The text a published range covers, as the editor would underline it. */
function covered(source: string, range: any): string {
  const lines = source.split("\n");
  const at = (p: { line: number; character: number }) =>
    lines.slice(0, p.line).reduce((n, l) => n + l.length + 1, 0) + p.character;
  return source.slice(at(range.start), at(range.end));
}

/** The diagnostics the server publishes for `source`, once it has some. */
async function published(source: string): Promise<any[]> {
  const { client, uri, stop } = await open(source);
  try {
    const params = await client.waitFor(
      "textDocument/publishDiagnostics",
      (p) => p.uri === uri && p.diagnostics.length > 0,
    );
    return params.diagnostics;
  } finally {
    stop();
  }
}

test(
  "the published diagnostic keeps its labels after the typed layer replaces the service one",
  { skip: skipTyped, timeout },
  async () => {
    // Under the default settings the typed pass replaces the
    // language-service layer wholesale, so the secondary labels must ride
    // the typed diagnostics or the final publish loses them.
    const source = [
      "const inc = (n: number): number => n + 1;",
      "const shout = (s: string): string => s.toUpperCase();",
      "const a = 1 |> inc |> shout;",
      "",
    ].join("\n");
    const diagnostics = await published(source);
    const mismatch = diagnostics.find(
      (d: any) => String(d.code ?? "") === "ts2345",
    );
    assert.ok(mismatch, JSON.stringify(diagnostics));
    const related = mismatch.relatedInformation ?? [];
    assert.equal(related.length, 1, JSON.stringify(mismatch));
    assert.equal(related[0].message, "the piped value is produced here");
    assert.equal(covered(source, related[0].location.range), "inc");
  },
);

test(
  "a new diagnostic generation never drops an untouched typed error",
  { skip: skipTyped, timeout },
  async () => {
    const source = [
      "val const scores = new Map<string, number>();",
      'scores.set("a", 1);',
      "function read(o: TOption<number>): number {",
      "  const Some(value) = o else {",
      '    console.log("missing");',
      "  };",
      "  return value;",
      "}",
      "",
    ].join("\n");
    const fixed = source.replace(
      '    console.log("missing");',
      "    return 0;",
    );
    const { client, uri, stop } = await open(source);
    try {
      await client.waitFor(
        "textDocument/publishDiagnostics",
        (p) =>
          p.uri === uri &&
          p.diagnostics.some((d: any) => d.code === "val-mutation") &&
          p.diagnostics.some(
            (d: any) => d.code === "let-else-not-diverging",
          ),
      );

      const firstNewGeneration = client.waitFor(
        "textDocument/publishDiagnostics",
        (p) => p.uri === uri,
      );
      client.notify("textDocument/didChange", {
        textDocument: { uri, version: 2 },
        contentChanges: [{ text: fixed }],
      });
      const published = await firstNewGeneration;

      assert.equal(published.version, 2);
      assert.ok(
        published.diagnostics.some((d: any) => d.code === "val-mutation"),
        `untouched typed error disappeared: ${JSON.stringify(published)}`,
      );
      assert.ok(
        !published.diagnostics.some(
          (d: any) => d.code === "let-else-not-diverging",
        ),
        `fixed text error remained: ${JSON.stringify(published)}`,
      );
    } finally {
      stop();
    }
  },
);

test(
  "a diagnostic underlines the construct it is about",
  { skip, timeout },
  async () => {
    const source = [
      "variant Shape { Circle(r: number), Square(s: number), Tri(a: number) }",
      "export function area(shape: Shape): number {",
      "  return match (shape) {",
      "    Circle(r) => r * r,",
      "    Square(s) => s * s,",
      "  };",
      "}",
      "",
    ].join("\n");
    const diagnostics = await published(source);
    const missing = diagnostics.find((d: any) =>
      d.message.includes("is not exhaustive"),
    );
    assert.ok(missing, `no exhaustiveness error in: ${JSON.stringify(diagnostics)}`);
    // The whole head, not just the word the position lands on.
    assert.equal(covered(source, missing.range), "match (shape)");
  },
);

test(
  "semantic pattern diagnostics keep their complete source spans",
  { skip, timeout },
  async () => {
    const source = [
      "variant Conn { Up(value: number), Down }",
      "variant Mode { Auto(), Manual }",
      "const mixed = match (c) { Up(value) => 1, 222 => 2, Down => 3 };",
      "const arity = match (c, m) { (Up(value)) => value, _ => 0 };",
      "",
    ].join("\n");
    const diagnostics = await published(source);
    const mixed = diagnostics.find(
      (d: any) => d.code === "match-mixed-patterns",
    );
    const arity = diagnostics.find(
      (d: any) => d.code === "match-tuple-arity",
    );
    assert.equal(covered(source, mixed?.range), "222");
    assert.equal(covered(source, arity?.range), "(Up(value))");
  },
);

test(
  "a construct that did not parse is reported where it is written",
  { skip, timeout },
  async () => {
    // Parser recovery owns this unambiguous tt near miss and keeps its
    // stable rule identity through the compiler protocol and LSP adapter.
    const source = [
      "variant Shape { Circle(r: number), Square(s: number) }",
      "export function area(shape: Shape): number {",
      "  return match shape {",
      "    Circle(r) => r,",
      "    Square(s) => s,",
      "  };",
      "}",
      "",
    ].join("\n");
    const diagnostics = await published(source);
    const failed = diagnostics.find((d: any) => d.code === "malformed-match");
    assert.ok(failed, `no parse report in: ${JSON.stringify(diagnostics)}`);
    assert.match(failed.message, /tt `match` could not be parsed/);
    // The fix travels as a suggestion with an edit, not as a sentence in
    // the message (TASK-218), so the editor can offer it as a quick fix.
    assert.deepEqual(failed.data?.suggestions?.[0]?.edit?.replacement, "(shape)");
    assert.equal(failed.range.start.line, 2);
    assert.equal(covered(source, failed.range), "match");
  },
);

test(
  "loop-header try is diagnosed at its expression boundary",
  { skip, timeout },
  async () => {
    const source = [
      "const a = () => Result.Err(10);",
      "",
      "function Test(): TResult<string, string> {",
      "  while (try a()) work();",
      "  return Result.Ok(\"done\");",
      "}",
      "",
    ].join("\n");
    const diagnostics = await published(source);
    const failed = diagnostics.find((d: any) => d.code === "try-placement");
    assert.ok(failed, `no placement error in: ${JSON.stringify(diagnostics)}`);
    assert.match(failed.message, /TypeScript control-flow boundary/);
    assert.equal(covered(source, failed.range), "try a()");
  },
);

test(
  "a misspelled case offers the compiler's own replacement as a quick fix",
  { skip, timeout },
  async () => {
    // The fix is the compiler's: the span and the text both arrive on the
    // diagnostic, so the editor applies an answer rather than guessing one
    // from the message (TASK-213).
    const source = [
      "variant Shape { Circle(radius: number), Empty }",
      "declare const s: Shape;",
      "const a = match (s) { Circel(radius) => radius, Empty => 0 };",
      "",
    ].join("\n");
    const { client, uri, stop } = await open(source);
    try {
      const params = await client.waitFor(
        "textDocument/publishDiagnostics",
        (p: any) =>
          p.uri === uri &&
          p.diagnostics.some((d: any) => d.code === "unknown-case"),
      );
      const diagnostic = params.diagnostics.find(
        (d: any) => d.code === "unknown-case",
      );
      // The message states the problem only — the fix travels beside it.
      assert.equal(diagnostic.message, "variant Shape has no case `Circel`");
      assert.equal(covered(source, diagnostic.range), "Circel");

      const response = await client.request("textDocument/codeAction", {
        textDocument: { uri },
        range: diagnostic.range,
        context: { diagnostics: [diagnostic] },
      });
      const actions = (response.result ?? []) as any[];
      // The action is titled with the compiler's own sentence, not with a
      // phrase the editor builds out of the replacement (TASK-216).
      const fix = actions.find(
        (a) => a.title === "a case with a similar name exists",
      );
      assert.ok(fix, `no replacement quick fix in: ${JSON.stringify(actions)}`);
      const edits = fix.edit.changes[uri];
      assert.equal(edits.length, 1);
      assert.equal(edits[0].newText, "Circle");
      assert.equal(covered(source, edits[0].range), "Circel");
    } finally {
      stop();
    }
  },
);

test(
  "a match with holes offers the arms the compiler wrote for it",
  { skip, timeout },
  async () => {
    const source = [
      "variant Shape { Circle(radius: number), Rect(w: number, h: number) }",
      "declare const s: Shape;",
      "const a = match (s) {",
      "  Circle(radius) => radius,",
      "};",
      "",
    ].join("\n");
    const { client, uri, stop } = await open(source);
    try {
      const params = await client.waitFor(
        "textDocument/publishDiagnostics",
        (p: any) =>
          p.uri === uri &&
          p.diagnostics.some((d: any) => d.code === "match-not-exhaustive"),
      );
      const diagnostic = params.diagnostics.find(
        (d: any) => d.code === "match-not-exhaustive",
      );
      const response = await client.request("textDocument/codeAction", {
        textDocument: { uri },
        range: diagnostic.range,
        context: { diagnostics: [diagnostic] },
      });
      const actions = (response.result ?? []) as any[];
      // Nothing here reads the message: the arms, their payload bindings
      // and the insertion point all arrive as an edit on the diagnostic,
      // so the extension has no rule-specific branch left (TASK-216).
      const armFix = actions.find((a) => a.title === "add the missing arms");
      assert.ok(armFix, `no arm quick fix in: ${JSON.stringify(actions)}`);
      const inserted = armFix.edit.changes[uri][0].newText;
      assert.match(inserted, /^ {2}Rect\(w, h\) => undefined,\n$/);
      const wildcard = actions.find(
        (a) => a.title === "or add a final `_` arm",
      );
      assert.ok(wildcard, "the wildcard fix is still offered");
      assert.match(
        wildcard.edit.changes[uri][0].newText,
        /^ {2}_ => undefined,\n$/,
      );
    } finally {
      stop();
    }
  },
);

/* ------------------------------------------------------------------ */
/* practical diagnostic matrix (TASK-308)                              */
/* ------------------------------------------------------------------ */

interface PracticalManifest {
  entry: string;
  diagnostics: Array<{
    code: string;
    text: string;
    line: number;
    message: string;
    help: string[];
    fix?: { title: string; text: string; replacement: string; fixed: string };
    labels: Array<{ text: string; line: number; message: string }>;
  }>;
}

const PRACTICAL_REPO_ROOT = path.resolve(
  __dirname,
  "..",
  "..",
  "..",
  "..",
  "..",
);
const PRACTICAL_FIXTURES = path.join(
  PRACTICAL_REPO_ROOT,
  "tests",
  "fixtures",
  "practical-diagnostics",
);

function stripPracticalAnnotations(source: string): string {
  return source
    .split("\n")
    .map((line) => {
      const marker = line.indexOf("//~");
      return marker < 0 ? line : line.slice(0, marker).trimEnd();
    })
    .join("\n");
}

function applyTextEdit(source: string, edit: any): string {
  const lines = source.split("\n");
  const at = (position: { line: number; character: number }) =>
    lines.slice(0, position.line).reduce((n, line) => n + line.length + 1, 0) +
    position.character;
  return (
    source.slice(0, at(edit.range.start)) +
    edit.newText +
    source.slice(at(edit.range.end))
  );
}

for (const caseName of fs.readdirSync(PRACTICAL_FIXTURES).sort()) {
  const fixture = path.join(PRACTICAL_FIXTURES, caseName);
  if (!fs.statSync(fixture).isDirectory()) continue;
  const manifest = JSON.parse(
    fs.readFileSync(path.join(fixture, "manifest.json"), "utf8"),
  ) as PracticalManifest;

  test(
    `the editor reports every practical diagnostic in ${caseName}`,
    { skip: skipTyped, timeout },
    async () => {
      const project = caseDir(`tt-practical-${caseName}-`);
      fs.cpSync(fixture, project, {
        recursive: true,
        filter: (source) => path.basename(source) !== "node_modules",
      });
      const file = path.join(project, manifest.entry);
      const source = stripPracticalAnnotations(fs.readFileSync(file, "utf8"));
      fs.writeFileSync(file, source);
      const uri = pathToFileURL(file).toString();
      const client = connect();
      try {
        await client.request("initialize", {
          processId: process.pid,
          rootUri: pathToFileURL(PRACTICAL_REPO_ROOT).toString(),
          workspaceFolders: [
            {
              uri: pathToFileURL(PRACTICAL_REPO_ROOT).toString(),
              name: "tt",
            },
          ],
          capabilities: {},
        });
        client.notify("initialized", {});
        client.notify("textDocument/didOpen", {
          textDocument: {
            uri,
            languageId: file.endsWith(".ttx") ? "ttx" : "tt",
            version: 1,
            text: source,
          },
        });

        const published = await client.waitFor(
          "textDocument/publishDiagnostics",
          (params) => params.uri === uri && params.diagnostics.length > 0,
        );
        const expectedCodes = manifest.diagnostics.map(({ code }) => code);
        const actualCodes = published.diagnostics.map((diagnostic: any) =>
          String(diagnostic.code),
        );
        assert.deepEqual(actualCodes, expectedCodes, JSON.stringify(published));

        for (const expected of manifest.diagnostics) {
          const diagnostic = published.diagnostics.find(
            (candidate: any) => String(candidate.code) === expected.code,
          );
          assert.ok(diagnostic, `missing ${expected.code}`);
          assert.equal(
            diagnostic.range.start.line + 1,
            expected.line,
            JSON.stringify(diagnostic),
          );
          assert.equal(covered(source, diagnostic.range), expected.text);
          assert.equal(diagnostic.message, expected.message);
          assert.deepEqual(
            (diagnostic.data?.suggestions ?? []).map(
              (suggestion: any) => suggestion.message,
            ),
            expected.help,
          );
          const related = diagnostic.relatedInformation ?? [];
          assert.deepEqual(
            related.map((label: any) => ({
              text: covered(source, label.location.range),
              line: label.location.range.start.line + 1,
              message: label.message,
            })),
            expected.labels,
          );
          if (expected.fix !== undefined) {
            const response = await client.request("textDocument/codeAction", {
              textDocument: { uri },
              range: diagnostic.range,
              context: { diagnostics: [diagnostic] },
            });
            const action = (response.result ?? []).find(
              (candidate: any) => candidate.title === expected.fix?.title,
            );
            assert.ok(action, JSON.stringify(response.result));
            const [edit] = action.edit.changes[uri];
            assert.equal(covered(source, edit.range), expected.fix.text);
            assert.equal(edit.newText, expected.fix.replacement);
            const fixed = applyTextEdit(source, edit);
            assert.equal(
              fixed,
              fs.readFileSync(path.join(project, expected.fix.fixed), "utf8"),
            );
            const remainingCodes = manifest.diagnostics
              .filter(({ code }) => code !== expected.code)
              .map(({ code }) => code);
            const republished = client.waitFor(
              "textDocument/publishDiagnostics",
              (params) =>
                params.uri === uri &&
                params.diagnostics.map((item: any) => String(item.code)).join() ===
                  remainingCodes.join(),
            );
            client.notify("textDocument/didChange", {
              textDocument: { uri, version: 2 },
              contentChanges: [{ text: fixed }],
            });
            await republished;
          }
        }
      } finally {
        client.stop();
        fs.rmSync(project, { recursive: true, force: true });
      }
    },
  );
}
