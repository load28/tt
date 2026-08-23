/* --------------------------------------------------------------------------
 * TextMate 문법 테스트 — syntaxes/tt.tmLanguage.json.
 *
 * 문법은 TypeScript 문법의 완전 확장(TSX 방식)으로 build.mjs가 생성한다.
 * 여기서 지키는 계약:
 *
 * 1. tt 구문은 **중첩 컨텍스트에서도** tt 스코프를 받는다 — 함수 본문,
 *    초기화식, 매개변수 목록. (기존 문법은 최상위에서만 동작해서 함수 안의
 *    모든 tt 구문이 순수 TS로 오해석됐고, `result { ... }`는 객체 리터럴로
 *    파싱되어 뒤 코드까지 연쇄 오염됐다.)
 * 2. 순수 TypeScript는 TypeScript 문법과 **동일한 스코프**를 받는다 —
 *    "유효한 TS는 그대로 유효한 tt" 통과 계약의 하이라이팅판.
 * 3. 생성물은 소스(src/)와 일치한다 — 손으로 고친 tt.tmLanguage.json은
 *    여기서 잡힌다.
 *
 * VS Code와 같은 엔진(vscode-textmate + oniguruma)으로 토크나이즈한다.
 * ----------------------------------------------------------------------- */
import { test } from "node:test";
import * as assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import * as fs from "node:fs";
import * as path from "node:path";
import * as oniguruma from "vscode-oniguruma";
import * as vsctm from "vscode-textmate";

const syntaxesDir = path.resolve(__dirname, "..", "..", "..", "syntaxes");

let registryPromise: Promise<vsctm.Registry> | undefined;
function registry(): Promise<vsctm.Registry> {
  registryPromise ??= (async () => {
    const wasmPath = path.join(
      path.dirname(require.resolve("vscode-oniguruma")),
      "onig.wasm",
    );
    const wasm = fs.readFileSync(wasmPath);
    await oniguruma.loadWASM(
      wasm.buffer.slice(wasm.byteOffset, wasm.byteOffset + wasm.byteLength),
    );
    return new vsctm.Registry({
      onigLib: Promise.resolve({
        createOnigScanner: (sources) => new oniguruma.OnigScanner(sources),
        createOnigString: (s) => new oniguruma.OnigString(s),
      }),
      loadGrammar: async (scopeName) => {
        const file =
          scopeName === "source.tt"
            ? path.join(syntaxesDir, "tt.tmLanguage.json")
            : scopeName === "source.ttx"
              ? path.join(syntaxesDir, "ttx.tmLanguage.json")
            : scopeName === "source.ts"
              ? path.join(syntaxesDir, "src", "typescript.tmLanguage.json")
              : scopeName === "source.tsx"
                ? path.join(syntaxesDir, "src", "typescriptreact.tmLanguage.json")
              : scopeName === "markdown.tt.codeblock"
                ? path.join(syntaxesDir, "tt.markdown.tmLanguage.json")
                : undefined;
        if (!file) return null;
        return vsctm.parseRawGrammar(fs.readFileSync(file, "utf8"), file);
      },
    });
  })();
  return registryPromise;
}

interface Token {
  text: string;
  scopes: string[];
}

async function tokenize(scopeName: string, source: string): Promise<Token[][]> {
  const grammar = await (await registry()).loadGrammar(scopeName);
  assert.ok(grammar, `grammar for ${scopeName}`);
  const lines: Token[][] = [];
  let stack = vsctm.INITIAL;
  for (const line of source.split("\n")) {
    const result = grammar.tokenizeLine(line, stack);
    lines.push(
      result.tokens.map((t) => ({
        text: line.slice(t.startIndex, t.endIndex),
        scopes: t.scopes,
      })),
    );
    stack = result.ruleStack;
  }
  return lines;
}

/** 1-기반 행 번호의 줄에서 text가 일치하는 첫 토큰을 찾는다. */
function tokenAt(lines: Token[][], line: number, text: string): Token {
  const hit = lines[line - 1]?.find((t) => t.text === text);
  assert.ok(
    hit,
    `no token ${JSON.stringify(text)} on line ${line}: ` +
      JSON.stringify(lines[line - 1]?.map((t) => t.text)),
  );
  return hit;
}

function assertScope(lines: Token[][], line: number, text: string, scope: string): void {
  const token = tokenAt(lines, line, text);
  assert.ok(
    token.scopes.includes(scope),
    `expected ${JSON.stringify(text)} on line ${line} to carry ${scope}, got: ${token.scopes.join(" ")}`,
  );
}

// 하이라이팅이 깨졌던 실제 리포트를 그대로 옮긴 픽스처.
const REPORTED = `export function endpoint(store: Record<string, string>): Result<string, ConfigError> {
  return result {
    const host <- get(store, "host");
    const portText <- get(store, "port");
    const port <- toPort(portText);
    \`https://\${host}:\${port}\`
  };
}

export function withTag(input: string[], tag: string): string {
  val const source = input;
  const next = [...source, tag];
  return next.join(", ");
}

function widen(val box: { width: number; height: number }): number {
  return box.width * box.height;
}
`;

test("tt constructs inside a function body get tt scopes (reported regression)", async () => {
  const lines = await tokenize("source.tt", REPORTED);

  // result 블록: 키워드, 바인딩, `<-` 전부 함수 본문 안에서.
  assertScope(lines, 2, "result", "keyword.control.result.tt");
  assertScope(lines, 3, "const", "storage.type.ts");
  assertScope(lines, 3, "host", "variable.other.readwrite.ts");
  assertScope(lines, 3, "<-", "keyword.operator.result-bind.tt");
  assertScope(lines, 3, "get", "entity.name.function.ts");
  assertScope(lines, 3, "host", "variable.other.readwrite.ts");
  // 블록의 마지막 값 식(템플릿)은 문자열로 남는다 — 객체 키가 아니라.
  assertScope(lines, 6, "https://", "string.template.ts");

  // result 블록이 객체 리터럴로 오해석되면 뒤 코드가 연쇄 오염된다 —
  // 다음 함수 시그니처가 온전하면 오염이 없는 것.
  assertScope(lines, 10, "withTag", "entity.name.function.ts");
  assertScope(lines, 10, "export", "keyword.control.export.ts");
  for (const token of lines.flat()) {
    assert.ok(
      !token.scopes.includes("meta.objectliteral.ts"),
      `object-literal misparse leaked into: ${JSON.stringify(token.text)}`,
    );
  }

  // val — 선언 수식자와 매개변수 수식자, 둘 다 중첩 위치에서.
  assertScope(lines, 11, "val", "storage.modifier.val.tt");
  assertScope(lines, 11, "source", "variable.other.constant.ts");
  assertScope(lines, 16, "val", "storage.modifier.val.tt");
  assertScope(lines, 16, "box", "variable.parameter.ts");
});

// 나머지 구문 전부를 함수 본문(중첩 컨텍스트)에 몰아넣은 픽스처.
const NESTED = `function demo(items: Item[]) {
  enum Shape<T> { Circle(r: number, meta?: T), Dot }
  const area = match (shape) {
    Circle(r) => r * r,
    Rect(w: width, h) if width > 0 => width * h,
    Dot | Empty => 0,
    "north" | "south" => 1,
    true => 2,
    Some(value: Circle(r)) => r,
    _ => -1,
  };
  const out = raw |> trim |> parse;
  const compose = flow |> trim |> parse;
  try cleanup();
  const parsed = try parse(raw);
  let Some(value: user) = findUser(id) else { return none; };
  if let Some(value: cached) = cache.get(id) {
    greet(cached);
  } else {
    prompt();
  }
  for (val const item of items) { log(item.id); }
  try { work(); } catch (val error: unknown) { log(error); }
  const r = result {
    const n: number <- toPort(raw);
    const { x, y } <- point();
    n + x + y
  };
  return r;
}
`;

test("every tt construct works in nested contexts", async () => {
  const lines = await tokenize("source.tt", NESTED);

  // enum — 함수 안 선언, 페이로드 필드와 타입.
  assertScope(lines, 2, "enum", "storage.type.enum.ts");
  assertScope(lines, 2, "Shape", "entity.name.type.enum.ts");
  assertScope(lines, 2, "Circle", "variable.other.enummember.ts");
  assertScope(lines, 2, "r", "variable.other.property.tt");
  assertScope(lines, 2, "number", "support.type.primitive.ts");
  assertScope(lines, 2, "Dot", "variable.other.enummember.ts");

  // match — 초기화식 위치, 패턴/가드/or-패턴/리터럴/와일드카드.
  assertScope(lines, 3, "match", "keyword.control.match.tt");
  assertScope(lines, 4, "Circle", "variable.other.enummember.ts");
  assertScope(lines, 4, "=>", "storage.type.function.arrow.ts");
  assertScope(lines, 5, "w", "variable.other.property.tt");
  assertScope(lines, 5, "width", "variable.other.readwrite.ts");
  assertScope(lines, 5, "if", "keyword.control.conditional.ts");
  assertScope(lines, 6, "|", "keyword.operator.or-pattern.tt");
  assertScope(lines, 7, "north", "string.quoted.double.ts");
  assertScope(lines, 8, "true", "constant.language.boolean.ts");
  assertScope(lines, 9, "Circle", "variable.other.enummember.ts");
  assertScope(lines, 10, "_", "keyword.control.wildcard.tt");

  // 파이프라인과 flow.
  assertScope(lines, 12, "|>", "keyword.operator.pipeline.tt");
  assertScope(lines, 13, "flow", "keyword.control.flow.tt");

  // try 문(식 형태)과 값 바인딩 형태.
  assertScope(lines, 14, "try", "keyword.control.trycatch.ts");
  assertScope(lines, 15, "try", "keyword.control.trycatch.ts");

  // let-else와 if let — 패턴, else, 본문.
  assertScope(lines, 16, "Some", "variable.other.enummember.ts");
  assertScope(lines, 16, "value", "variable.other.property.tt");
  assertScope(lines, 16, "user", "variable.other.readwrite.ts");
  assertScope(lines, 16, "else", "keyword.control.conditional.ts");
  assertScope(lines, 16, "return", "keyword.control.flow.ts");
  assertScope(lines, 17, "if", "keyword.control.conditional.ts");
  assertScope(lines, 17, "let", "storage.type.ts");
  assertScope(lines, 17, "Some", "variable.other.enummember.ts");
  assertScope(lines, 18, "greet", "entity.name.function.ts");

  // val — for 헤드와 catch 매개변수.
  assertScope(lines, 22, "val", "storage.modifier.val.tt");
  assertScope(lines, 23, "val", "storage.modifier.val.tt");
  assertScope(lines, 23, "error", "variable.parameter.ts");
  assertScope(lines, 23, "unknown", "support.type.primitive.ts");

  // result — 타입 주석 바인딩과 구조 분해 바인딩.
  assertScope(lines, 24, "result", "keyword.control.result.tt");
  assertScope(lines, 25, "n", "variable.other.readwrite.ts");
  assertScope(lines, 25, "number", "support.type.primitive.ts");
  assertScope(lines, 25, "<-", "keyword.operator.result-bind.tt");
  assertScope(lines, 26, "x", "variable.other.readwrite.ts");
  assertScope(lines, 26, "<-", "keyword.operator.result-bind.tt");
});

// tt 구문과 형태가 겹치는 순수 TypeScript — tt로 오인하면 안 된다.
const LOOKALIKES = `const enum Flags { A = 1, B = 2 }
function passthrough(a: number, b: number) {
  const cmp = a < -b;
  const box = { result: 1 };
  return cmp;
}
`;

test("TypeScript look-alikes are not claimed by tt rules", async () => {
  const lines = await tokenize("source.tt", LOOKALIKES);
  // const enum은 TS enum 규칙의 것 — tt enum 스코프가 없어야 한다.
  // (루트 스코프 source.tt은 모든 토큰이 가지므로 제외.)
  for (const token of lines[0]) {
    assert.ok(
      !token.scopes.slice(1).some((s) => s.endsWith(".tt")),
      `const enum leaked a tt scope on ${JSON.stringify(token.text)}`,
    );
  }
  // `a < -b`는 비교 — result 바인딩이 아니다.
  assertScope(lines, 3, "<", "keyword.operator.relational.ts");
  // 객체 키 result는 키워드가 아니다.
  const key = tokenAt(lines, 4, "result");
  assert.ok(!key.scopes.includes("keyword.control.result.tt"));
});

// 순수 TypeScript 픽스처 — tt 문법과 TS 문법의 스코프가 같아야 한다.
const PURE_TS = `import { readFile } from "node:fs/promises";

export interface User { name: string; tags: string[]; }
export type Outcome<T, E> = { ok: true; value: T } | { ok: false; error: E };

export class Repo<T extends User> {
  private cache = new Map<string, T>();
  constructor(private readonly root: string) {}
  async load(id: string): Promise<T | undefined> {
    try {
      const raw = await readFile(\`\${this.root}/\${id}.json\`, "utf8");
      return JSON.parse(raw) as T;
    } catch (error: unknown) {
      console.error("load failed", error);
      return undefined;
    }
  }
}

export function totals(xs: number[]): number {
  let sum = 0;
  for (const x of xs) sum += x;
  const doubled = xs.map((x) => x * 2).filter((x) => x > 0);
  return doubled.reduce((a, b) => a + b, sum);
}

const label = totals([1, 2, 3]) > 3 ? "big" : "small";
export default { label };
`;

test("pure TypeScript tokenizes identically to the TypeScript grammar", async () => {
  const tt = await tokenize("source.tt", PURE_TS);
  const ts = await tokenize("source.ts", PURE_TS);
  const strip = (lines: Token[][]) =>
    lines.map((line) =>
      line.map((t) => ({ text: t.text, scopes: t.scopes.slice(1).join(" ") })),
    );
  assert.deepEqual(strip(tt), strip(ts));
});

const PURE_TSX = `type Props<T> = { value: T };
export const Item = <T,>({ value }: Props<T>) => (
  <main data-kind="item"><strong>{String(value)}</strong></main>
);
`;

test("pure TSX tokenizes identically to the TypeScript React grammar", async () => {
  const ttx = await tokenize("source.ttx", PURE_TSX);
  const tsx = await tokenize("source.tsx", PURE_TSX);
  const strip = (lines: Token[][]) =>
    lines.map((line) =>
      line.map((t) => ({ text: t.text, scopes: t.scopes.slice(1).join(" ") })),
    );
  assert.deepEqual(strip(ttx), strip(tsx));
});

test("tt constructs inside JSX expression containers keep tt scopes", async () => {
  const lines = await tokenize(
    "source.ttx",
    `enum State { Ready(value: string), Empty }
export const View = ({ state }: { state: State }) => (
  <main>{match (state) { Ready(value) => <b>{value}</b>, Empty => null }}</main>
);`,
  );
  assertScope(lines, 3, "main", "entity.name.tag.tsx");
  assertScope(lines, 3, "match", "keyword.control.match.tt");
});

test("the extension registers ttx and shares each file icon across themes", () => {
  const extensionDir = path.join(syntaxesDir, "..");
  const pkg = JSON.parse(
    fs.readFileSync(path.join(extensionDir, "package.json"), "utf8"),
  );
  assert.ok(pkg.activationEvents.includes("onLanguage:ttx"));

  for (const [id, extension] of [["tt", ".tt"], ["ttx", ".ttx"]]) {
    const language = pkg.contributes.languages.find(
      (entry: { id: string }) => entry.id === id,
    );
    assert.ok(language, `package.json contributes no ${id} language`);
    assert.deepEqual(language.extensions, [extension]);
    assert.equal(language.icon.light, `./icons/${id}-file.svg`);
    assert.equal(language.icon.dark, language.icon.light);
    assert.ok(fs.existsSync(path.join(extensionDir, language.icon.light)));
  }

  const grammar = pkg.contributes.grammars.find(
    (entry: { language?: string }) => entry.language === "ttx",
  );
  assert.deepEqual(grammar, {
    language: "ttx",
    scopeName: "source.ttx",
    path: "./syntaxes/ttx.tmLanguage.json",
  });
});

// 마크다운 injection 문법 — ```tt 펜스 안을 source.tt로 칠한다.
// 호스트(내장 마크다운 문법)를 vendoring하지 않고 injection 문법을 루트로
// 직접 토크나이즈한다 — 펜스 인식·source.tt 임베드·펜스 종료·타 언어 펜스
// 불간섭이 이 문법의 계약 전부다 (TASK-094 결정 3).
const MARKDOWN_FENCES = `\`\`\`tt
enum Shape { Circle(r: number), Dot }
const area = match (s) { Circle(r) => r * r, _ => 0 };
\`\`\`
after the fence

~~~tt
const out = raw |> trim |> parse;
~~~

\`\`\`ts
const enum Flags { A = 1 }
\`\`\`
`;

test("markdown ```tt fences embed the tt grammar", async () => {
  const lines = await tokenize("markdown.tt.codeblock", MARKDOWN_FENCES);

  // 여는 펜스: 구분자와 언어 태그가 내장 마크다운과 같은 스코프를 받는다.
  assertScope(lines, 1, "```", "punctuation.definition.markdown");
  assertScope(lines, 1, "tt", "fenced_code.block.language.markdown");

  // 펜스 안: 내용이 meta.embedded.block.tt로 감싸이고 tt/TS 스코프가 붙는다.
  assertScope(lines, 2, "enum", "storage.type.enum.ts");
  assertScope(lines, 2, "Circle", "variable.other.enummember.ts");
  assertScope(lines, 3, "match", "keyword.control.match.tt");
  assertScope(lines, 3, "_", "keyword.control.wildcard.tt");
  for (const line of [2, 3]) {
    for (const token of lines[line - 1]) {
      assert.ok(
        token.scopes.includes("meta.embedded.block.tt"),
        `line ${line} token ${JSON.stringify(token.text)} is outside the embedded block`,
      );
    }
  }

  // 닫는 펜스에서 임베드가 끝난다 — 뒤 텍스트는 tt로 칠해지지 않는다.
  assertScope(lines, 4, "```", "punctuation.definition.markdown");
  for (const token of lines[5 - 1]) {
    assert.ok(
      token.scopes.length === 1,
      `text after the fence leaked scopes: ${token.scopes.join(" ")}`,
    );
  }

  // ~~~ 펜스도 동일하게 동작한다.
  assertScope(lines, 8, "|>", "keyword.operator.pipeline.tt");

  // 다른 언어의 펜스는 건드리지 않는다.
  for (const line of [11, 12, 13]) {
    for (const token of lines[line - 1]) {
      assert.ok(
        token.scopes.length === 1,
        `\`\`\`ts fence was claimed on line ${line}: ${token.scopes.join(" ")}`,
      );
    }
  }
});

test("markdown injection targets markdown and MDX hosts consistently", () => {
  // injection 문법의 selector와 package.json의 injectTo는 같은 호스트 집합을
  // 가리켜야 한다 — 한쪽만 고치면 에디터에서 주입이 조용히 빠진다.
  const hosts = ["text.html.markdown", "source.mdx"];
  const grammar = JSON.parse(
    fs.readFileSync(path.join(syntaxesDir, "tt.markdown.tmLanguage.json"), "utf8"),
  );
  assert.equal(grammar.injectionSelector, hosts.map((h) => `L:${h}`).join(", "));
  const pkg = JSON.parse(
    fs.readFileSync(path.join(syntaxesDir, "..", "package.json"), "utf8"),
  );
  const entry = pkg.contributes.grammars.find(
    (g: { scopeName: string }) => g.scopeName === grammar.scopeName,
  );
  assert.ok(entry, `package.json contributes no grammar for ${grammar.scopeName}`);
  assert.deepEqual(entry.injectTo, hosts);
  assert.deepEqual(entry.embeddedLanguages, { "meta.embedded.block.tt": "tt" });
});

test("generated grammar matches its sources (build.mjs --check)", () => {
  execFileSync(process.execPath, [path.join(syntaxesDir, "build.mjs"), "--check"], {
    stdio: "pipe",
  });
});
