import { variant } from "./variants.mjs";

const shape = variant("Shape", [
  ["Circle", [["r", "number"]]],
  ["Rect", [["w", "number"], ["h", "number"]]],
  ["Point", null],
]);
const shapes = "[Shape.Circle(2), Shape.Rect(3, 4), Shape.Point]";

const disk = variant("Disk", [["Round", [["radius", "number"]]], ["Dot", null]]);
const disks = "[Disk.Round(2), Disk.Dot]";

const opt = variant("Maybe", [
  ["Some", [["value", "number"]]],
  ["None", null],
]);
const outcome = variant("Outcome", [
  ["Done", [["value", "Maybe"]]],
  ["Failed", [["error", "string"]]],
]);
const outcomes = '[Outcome.Done(Maybe.Some(7)), Outcome.Done(Maybe.None), Outcome.Failed("lost")]';

const dir = variant("Dir", [["Up", null], ["Down", null]]);

const result = (body) => (x, t, c) => c.iife(`try {\n${body(x)}\n} catch (error) {\nreturn caught(error);\n}`);

const read = `declare function read(n: number): { kind: "Ok"; value: number } | { kind: "Err"; error: string };`;

export const extraPositions = [
  {
    id: "enumMember",
    title: "an enum member initializer",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) => c.probe(`enum Holder {\nfirst = ${c.v},\n}\nreturn ${c.ret("Holder.first")};`),
  },
  {
    id: "computedKey",
    title: "a class member's computed name",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) => c.probe(`class Holder {\n[${c.v}] = "keyed";\n}\nreturn ${c.ret("Object.keys(new Holder())")};`),
  },
  {
    id: "decorator",
    title: "a class decorator's argument",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) => c.probe(`@decorate(${c.v})\nclass Holder {}\nreturn ${c.ret("Holder.name")};`),
  },
  {
    id: "decoratedHeritage",
    title: "the heritage of a decorated class",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) => c.probe(`@decorate("class")\nclass Holder extends heritage(${c.v}) {}\nreturn ${c.ret("new Holder().inherited")};`),
  },
  {
    id: "forLaterDeclarator",
    title: "a later declarator of a C-style for head",
    rejects: { match: "match-placement", try: "try-placement" },
    host: (c) =>
      c.probe(
        `const collected: unknown[] = [];\nfor (let round = 0, item = ${c.v}; round < 1; round++) {\ncollected.push(item);\n}\nreturn ${c.ret("collected")};`,
      ),
  },
];

const numberMatch = {
  family: "match",
  kind: "value",
  jsx: true,
  In: "number",
  inputs: "[1, 2]",
};

export const diagnostics = [
  {
    code: "stray-pipe",
    examples: [
      {
        id: "arrowStep",
        title: "an unparenthesized arrow function as a step",
        fixTitle: "the arrow function parenthesized",
        family: "pipeline",
        kind: "value",
        jsx: true,
        lowPrecedence: true,
        In: "number",
        inputs: "[1, 2]",
        bad: (x) => `${x} [||>|] (n: number) => note("step", n + 1)`,
        tt: (x) => `${x} |> ((n: number) => note("step", n + 1))`,
        ts: (x) => `((n: number) => note("step", n + 1))(${x})`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `declare const x: number;\nexport const next = () => {\n  const n = x [||>|] n => n + 1;\n  return n;\n};`,
      },
      {
        role: "fix",
        units: `declare const ready: boolean;\ndeclare const a: number, b: number, x: number;\ndeclare function f(n: number): number;\nexport function both() {\n  (ready ? a : b) |> f\n  x |> (n => n + 1)\n}`,
      },
    ],
  },
  {
    code: "malformed-pipeline-postfix",
    examples: [
      {
        id: "taggedTemplate",
        title: "an optional postfix step that ends in a tagged template",
        fixTitle: "the step ending in a call",
        family: "pipeline",
        kind: "value",
        jsx: true,
        lowPrecedence: true,
        In: "number",
        inputs: "[1.25, 2]",
        bad: (x) => `${x} |> ?.toFixed[|\`1\`|]`,
        tt: (x) => `${x} |> ?.toFixed(1)`,
        ts: (x) => `(${x})?.toFixed(1)`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function label(value: { name: string } | undefined) {\n  return value |> ?.name.trim[|\`x\`|];\n}`,
      },
    ],
  },
  {
    code: "missing-pipeline-step",
    examples: [
      {
        id: "trailingPipe",
        title: "a pipeline that ends in `|>`",
        fixTitle: "the trailing `|>` removed",
        family: "pipeline",
        kind: "value",
        jsx: true,
        lowPrecedence: true,
        In: "number",
        inputs: "[1, 2]",
        bad: (x) => `${x} |> String [||>|]`,
        tt: (x) => `${x} |> String`,
        ts: (x) => `String(${x})`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function size(xs: string[]) {\n  const n = xs |> .length [||>|]\n  return n;\n}`,
      },
    ],
  },
  {
    code: "invalid-optional-receiver",
    examples: [
      {
        id: "superReceiver",
        title: "bare `super` piped into an optional call",
        fixTitle: "an ordinary `super.member` call before the pipeline",
        kind: "module",
        surfaces: ["tt", "ttx"],
        run: true,
        bad: {
          "main.EXT": `class Base {\n  label() {\n    return "base";\n  }\n}\nexport class Derived extends Base {\n  override label() {\n    return [|super|] |> ?.label();\n  }\n}`,
        },
        good: {
          "main.EXT": `class Base {\n  label() {\n    return "base";\n  }\n}\nexport class Derived extends Base {\n  override label() {\n    return super.label() |> ((s: string) => s.toUpperCase());\n  }\n}\nconsole.log(new Derived().label());`,
          "twin.TS": `class Base {\n  label() {\n    return "base";\n  }\n}\nexport class Derived extends Base {\n  override label() {\n    return ((s: string) => s.toUpperCase())(super.label());\n  }\n}\nconsole.log(new Derived().label());`,
        },
      },
    ],
    explain: [
      {
        role: "error",
        units: `class Base {\n  label() {\n    return "base";\n  }\n}\nexport class Derived extends Base {\n  override label() {\n    return [|super|] |> ?.label();\n  }\n}`,
      },
    ],
  },
  {
    code: "stray-if-let",
    examples: [
      {
        id: "elseIf",
        title: "an `if let` continued by a plain `else if`",
        fixTitle: "the plain `if` inside an `else` block",
        family: "ifLet",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[2, -1]",
        temps: 1,
        result: "seen",
        bad: (x) => `let seen: unknown = "none";\nif let Ok(value: v) = read(${x}) { seen = note("then", v); } [|else if|] (flip()) { seen = "flip"; }`,
        tt: (x) => `let seen: unknown = "none";\nif let Ok(value: v) = read(${x}) { seen = note("then", v); } else { if (flip()) { seen = "flip"; } }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const v = ${t}.value; seen = note("then", v); } else { if (flip()) { seen = "flip"; } } }`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function first(n: number, fallback: boolean) {\n  if let Ok(value: v) = read(n) {\n    return v;\n  } [|else if|] (fallback) {\n    return 0;\n  }\n  return -1;\n}`,
      },
    ],
  },
  {
    code: "malformed-variant",
    examples: [
      {
        id: "missingComma",
        title: "a variant whose cases are not separated by commas",
        fixTitle: "the cases separated by commas",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export [|variant|] Shape { Circle(r: number) Point }` },
        good: { "main.EXT": `export variant Shape { Circle(r: number), Point }` },
      },
    ],
    explain: [
      {
        role: "error",
        units: `[|variant|] Shape { Circle(r: number) Point }`,
      },
    ],
  },
  {
    code: "malformed-match",
    examples: [
      {
        ...numberMatch,
        id: "emptyScrutinee",
        title: "a match with an empty scrutinee",
        fixTitle: "the scrutinee written",
        bad: () => `[|match|] () { 1 => note("one", 1), _ => note("other", 0) }`,
        tt: (x) => `match (${x}) { 1 => note("one", 1), _ => note("other", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("one", 1) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function describe(code: number) {\n  return [|match|] () { 200 => "ok", _ => "other" };\n}`,
      },
    ],
  },
  {
    code: "missing-arm-body",
    examples: [
      {
        ...numberMatch,
        id: "noBody",
        title: "a final arm with no body after its `=>`",
        fixTitle: "the arm's body written",
        bad: (x) => `match (${x}) { 1 => note("one", 1), [|_|] => }`,
        tt: (x) => `match (${x}) { 1 => note("one", 1), _ => note("other", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("one", 1) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(radius: number), Rect(width: number) }\nexport function size(shape: Shape) {\n  return (\n    match (shape) {\n      Circle(radius) => radius,\n      [|_|] =>\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "result-no-success-value",
    examples: [
      {
        id: "fallsOff",
        title: "a result block whose body can fall off its end",
        fixTitle: "a return on the path that fell off",
        family: "result",
        kind: "value",
        jsx: true,
        In: "number",
        inputs: "[3, 1, -1]",
        bad: (x) => `[|result { const n = try read(${x}); if (n > 2) { return note("big", n); } }|]`,
        tt: (x) => `result { const n = try read(${x}); if (n > 2) { return note("big", n); } return note("small", n); }`,
        ts: result((x) => `const n = unwrap(read(${x})); if (n > 2) { return ok(note("big", n)); } return ok(note("small", n));`),
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function parse(text: string) {\n  const r = [|result {\n    const n = try read(text.length);\n    if (n > 0) {\n      return n;\n    }\n  }|];\n  return r;\n}`,
      },
    ],
  },
  {
    code: "result-value-discarded",
    examples: [
      {
        id: "statement",
        title: "a result block written as an expression statement",
        fixTitle: "the Result stored in a binding",
        family: "result",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[2, -1]",
        result: "kept",
        bad: (x) => `let kept: unknown = "none";\n[|result { const n = try read(${x}); return note("n", n); }|];`,
        tt: (x) => `let kept: unknown = "none";\nkept = result { const n = try read(${x}); return note("n", n); };`,
        ts: (x, t, exit, c) =>
          `let kept: unknown = "none";\nkept = ${c.iife(`try {\nconst n = unwrap(read(${x})); return ok(note("n", n));\n} catch (error) {\nreturn caught(error);\n}`)};`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function check(n: number) {\n  [|result {\n    const v = try read(n);\n    return v;\n  }|];\n}`,
      },
    ],
  },
  {
    code: "result-return-nested",
    examples: [
      {
        id: "returnsResult",
        title: "a result block returning a Result",
        fixTitle: "the returned Result unwrapped with `try`",
        family: "result",
        kind: "value",
        jsx: true,
        typedOnly: true,
        In: "number",
        inputs: "[2, 0, -1]",
        bad: (x) => `result { const n = try read(${x}); return [|read(n - 1)|]; }`,
        tt: (x) => `result { const n = try read(${x}); return try read(n - 1); }`,
        ts: result((x) => `const n = unwrap(read(${x})); return ok(unwrap(read(n - 1)));`),
      },
    ],
    explain: [
      {
        role: "error",
        typedOnly: true,
        units: `${read}\nexport function twice(n: number) {\n  return result {\n    const v = try read(n);\n    return [|read(v * 2)|];\n  };\n}`,
      },
    ],
  },
  {
    code: "result-break-crossing",
    examples: [
      {
        id: "breakOut",
        title: "a break in a result block that leaves the loop around it",
        fixTitle: "the break moved after the block",
        family: "result",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[3, -1]",
        result: "seen",
        bad: (x) =>
          `let seen: unknown = "none";\nfor (const round of [1, 2]) { const r = result { const n = try read(${x}); if (n > round) [|break|]; return n; }; seen = note("r", r); }`,
        tt: (x) =>
          `let seen: unknown = "none";\nfor (const round of [1, 2]) { const r = result { const n = try read(${x}); return n; }; if (r.kind === "Ok" && r.value > round) break; seen = note("r", r); }`,
        ts: (x, t, exit, c) =>
          `let seen: unknown = "none";\nfor (const round of [1, 2]) { const r = ${c.iife(`try {\nconst n = unwrap(read(${x})); return ok(n);\n} catch (error) {\nreturn caught(error);\n}`)}; if (r.kind === "Ok" && r.value > round) break; seen = note("r", r); }`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function first(xs: number[]) {\n  for (const x of xs) {\n    const r = result {\n      const v = try read(x);\n      if (v > 9) [|break|];\n      return v;\n    };\n  }\n}`,
      },
    ],
  },
  {
    code: "result-continue-crossing",
    examples: [
      {
        id: "continueOut",
        title: "a continue in a result block that goes on with the loop around it",
        fixTitle: "the continue moved after the block",
        family: "result",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[3, -1]",
        result: "seen",
        bad: (x) =>
          `let seen: unknown[] = [];\nfor (const round of [1, 2]) { const r = result { const n = try read(${x}); if (n > round) [|continue|]; return n; }; seen.push(note("r", r)); }`,
        tt: (x) =>
          `let seen: unknown[] = [];\nfor (const round of [1, 2]) { const r = result { const n = try read(${x}); return n; }; if (r.kind === "Ok" && r.value > round) continue; seen.push(note("r", r)); }`,
        ts: (x, t, exit, c) =>
          `let seen: unknown[] = [];\nfor (const round of [1, 2]) { const r = ${c.iife(`try {\nconst n = unwrap(read(${x})); return ok(n);\n} catch (error) {\nreturn caught(error);\n}`)}; if (r.kind === "Ok" && r.value > round) continue; seen.push(note("r", r)); }`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function each(xs: number[]) {\n  for (const x of xs) {\n    const r = result {\n      const v = try read(x);\n      if (v > 9) [|continue|];\n      return v;\n    };\n  }\n}`,
      },
    ],
  },
  {
    code: "result-yield-crossing",
    examples: [
      {
        id: "yieldInside",
        title: "a yield in a result block inside a generator",
        fixTitle: "the yield moved after the block",
        family: "result",
        kind: "value",
        jsx: true,
        In: "number",
        inputs: "[3, -1]",
        skip: ["conditionalTest"],
        bad: (x) => `[...(function* () { const r = result { const n = try read(${x}); [|yield|] n; return n; }; yield r; })()]`,
        tt: (x) => `[...(function* () { const r = result { const n = try read(${x}); return n; }; if (r.kind === "Ok") yield r.value; yield r; })()]`,
        ts: (x, t, c) =>
          `[...(function* () { const r = ${c.iife(`try {\nconst n = unwrap(read(${x})); return ok(n);\n} catch (error) {\nreturn caught(error);\n}`)}; if (r.kind === "Ok") yield r.value; yield r; })()]`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function* values(xs: number[]) {\n  for (const x of xs) {\n    const r = result {\n      const v = try read(x);\n      [|yield|] v;\n      return v;\n    };\n  }\n}`,
      },
    ],
  },
  {
    code: "result-label-crossing",
    examples: [
      {
        id: "labeledBreak",
        title: "a labeled break in a result block that leaves the labeled block around it",
        fixTitle: "the labeled break moved after the block",
        family: "result",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[3, -1]",
        result: "seen",
        bad: (x) =>
          `let seen: unknown = "none";\nouter: { const r = result { const n = try read(${x}); if (n > 1) [|break|] outer; return n; }; seen = note("r", r); }`,
        tt: (x) =>
          `let seen: unknown = "none";\nouter: { const r = result { const n = try read(${x}); return n; }; if (r.kind === "Ok" && r.value > 1) break outer; seen = note("r", r); }`,
        ts: (x, t, exit, c) =>
          `let seen: unknown = "none";\nouter: { const r = ${c.iife(`try {\nconst n = unwrap(read(${x})); return ok(n);\n} catch (error) {\nreturn caught(error);\n}`)}; if (r.kind === "Ok" && r.value > 1) break outer; seen = note("r", r); }`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function scan(x: number) {\n  found: {\n    const r = result {\n      const v = try read(x);\n      if (v > 9) [|break|] found;\n      return v;\n    };\n  }\n}`,
      },
    ],
  },
  {
    code: "flow-first-step-method",
    examples: [
      {
        id: "methodFirst",
        title: "a flow whose first step is a method step",
        fixTitle: "a function first",
        family: "flow",
        kind: "value",
        lowPrecedence: true,
        skip: ["templateLiteral", "compoundAssignment", "conditionalTest"],
        In: "number",
        inputs: "[1.25, 2]",
        bad: () => `flow |> [|.toFixed(1)|] |> String`,
        tt: () => `flow |> ((n: number) => n.toFixed(1)) |> String`,
        ts: () => `((v: number) => String(((n: number) => n.toFixed(1))(v)))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `const label = flow |> [|.toFixed(1)|] |> String;\nexport { label };`,
      },
      {
        role: "fix",
        units: `const half = (n: number) => n / 2;\nconst scale = { by: (n: number) => n * 3 } as { by(n: number): number } | undefined;\nexport function make() {\n  const label = flow |> half |> .toFixed(1);\n  const scaled = flow |> ((n: number) => scale?.by(n)) |> String;\n  return [label, scaled];\n}`,
      },
    ],
  },
  {
    code: "try-placement",
    examples: [
      {
        id: "value",
        title: "the value form where its propagation has no statement position",
        fixTitle: "the value lifted into a declaration before the host",
        family: "try",
        where: "rejects",
        kind: "value",
        target: true,
        In: "number",
        inputs: "[2, -1]",
        bad: (x) => `[|try read(${x})|]`,
        ts: (x) => `unwrap(read(${x}))`,
        fixes: {
          yieldOperand: {
            fixTitle: "a result block that owns the propagation",
            target: false,
            tt: (x) => `result { return try read(${x}); }`,
            ts: result((x) => `return ok(unwrap(read(${x})));`),
          },
          topLevel: {
            fixTitle: "a result block that owns the propagation",
            target: false,
            tt: (x) => `result { return try read(${x}); }`,
            ts: result((x) => `return ok(unwrap(read(${x})));`),
          },
        },
      },
      {
        id: "statement",
        title: "the statement form where its propagation has no function to return from",
        fixTitle: "the statement moved into a function",
        family: "try",
        where: "rejects",
        kind: "statement",
        target: true,
        In: "number",
        inputs: "[5, -5]",
        result: "checked",
        hoisted: '"checked"',
        unbraced: "try-placement",
        bad: (x) => `const checked = [|try read(${x})|];`,
        bads: { unbracedIf: (x) => `[|const checked = try read(${x});|]` },
        ts: (x) => `const checked = unwrap(read(${x}));`,
        fixes: {
          unbracedIf: {
            fixTitle: "the declaration wrapped in braces",
            tt: (x) => `{ const checked = try read(${x}); note("checked", checked); }`,
            ts: (x) => `{ const checked = unwrap(read(${x})); note("checked", checked); }`,
          },
          topLevel: {
            fixTitle: "a result block that owns the propagation",
            target: false,
            tt: (x) => `const checked = result { return try read(${x}); };`,
            ts: (x, t, exit, c) => `const checked = ${c.iife(`try {\nreturn ok(unwrap(read(${x})));\n} catch (error) {\nreturn caught(error);\n}`)};`,
          },
        },
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function total(xs: number[]) {\n  let sum = 0;\n  for (let i = 0; i < [|try read(xs.length)|]; i++) {\n    sum += xs[i]!;\n  }\n  return sum;\n}`,
      },
    ],
  },
  {
    code: "try-crosses-value-region",
    examples: [
      {
        id: "templateInterpolation",
        title: "a try in a template interpolation inside a result block",
        fixTitle: "the value unwrapped into a binding before the template",
        family: "result",
        kind: "value",
        jsx: true,
        In: "number",
        inputs: "[2, -1]",
        bad: (x) => `result { return \`<\${[|try read(${x})|]}>\`; }`,
        tt: (x) => `result { const n = try read(${x}); return \`<\${n}>\`; }`,
        ts: result((x) => `const n = unwrap(read(${x})); return ok(\`<\${n}>\`);`),
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function show(n: number) {\n  return result { return \`value: \${[|try read(n)|]}\`; };\n}`,
      },
    ],
  },
  {
    code: "let-else-placement",
    examples: [
      {
        id: "lexical",
        title: "a const let-else where no statement list owns its exits",
        fixTitle: "the let-else moved into the function's statement list",
        family: "letElse",
        where: "rejects",
        kind: "statement",
        In: "number",
        inputs: "[2, -1]",
        temps: 1,
        result: "v",
        hoisted: '"bound"',
        unbraced: "let-else-placement",
        bad: (x, t, exit) => `[|const Ok(value: v) = read(${x})|] else { ${exit} };`,
        ts: (x, [t], exit) => `const ${t} = read(${x});\nif (${t}.kind !== "Ok") { ${exit} }\nconst v = ${t}.value;`,
        fixes: {
          unbracedIf: {
            fixTitle: "the let-else wrapped in braces",
            tt: (x, t, exit) => `{ const Ok(value: v) = read(${x}) else { ${exit} }; note("bound", v); }`,
            ts: (x, [t], exit) => `{ const ${t} = read(${x});\nif (${t}.kind !== "Ok") { ${exit} }\nconst v = ${t}.value; note("bound", v); }`,
          },
        },
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(r: number), Point }\nexport function radius(s: Shape, round: boolean) {\n  if (round) [|const Circle(r) = s|] else { return 0; };\n  return 1;\n}`,
      },
    ],
  },
  {
    code: "let-else-not-diverging",
    examples: [
      {
        id: "fallsThrough",
        title: "a let-else whose else block falls out of its bottom",
        fixTitle: "an else block that leaves",
        family: "letElse",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[2, -1]",
        temps: 1,
        result: "v",
        bad: (x) => `const Ok(value: v) = read(${x}) [|else|] { note("else", "fell"); };`,
        tt: (x, t, exit) => `const Ok(value: v) = read(${x}) else { ${exit} };`,
        ts: (x, [t], exit) => `const ${t} = read(${x});\nif (${t}.kind !== "Ok") { ${exit} }\nconst v = ${t}.value;`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function parse(n: number) {\n  const Ok(value: v) = read(n) [|else|] {\n    console.log("not a number");\n  };\n  return v;\n}`,
      },
    ],
  },
  {
    code: "if-let-placement",
    examples: [
      {
        id: "asValue",
        title: "an if-let written as a value",
        fixTitle: "a variable declared before the statement and assigned in its bodies",
        family: "ifLet",
        where: "all",
        kind: "value",
        jsx: true,
        In: "number",
        inputs: "[2, -1]",
        temps: 1,
        bad: (x) => `[|if let Ok(value: n) = read(${x})|] { note("then", n) } else { note("else", 0) }`,
        ts: () => "lifted",
        lift: {
          tt: (x, t, name) =>
            `let ${name}: number;\nif let Ok(value: n) = read(${x}) { ${name} = note("then", n); } else { ${name} = note("else", 0); }`,
          ts: (x, [t], name) =>
            `let ${name}: number;\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const n = ${t}.value; ${name} = note("then", n); } else { ${name} = note("else", 0); } }`,
        },
      },
    ],
    explain: [
      {
        role: "error",
        units: `${read}\nexport function parse(n: number) {\n  const v = [|if let Ok(value: x) = read(n)|] { x } else { 0 };\n  return v;\n}`,
      },
    ],
  },
  {
    code: "variant-duplicate-case",
    examples: [
      {
        id: "repeatedTag",
        title: "a variant declaring one tag twice",
        fixTitle: "the second case renamed",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export variant Shape { Circle(r: number), [|Circle|] }` },
        good: { "main.EXT": `export variant Shape { Circle(r: number), Point }` },
      },
    ],
    explain: [{ role: "error", units: `variant Token { Word(text: string), Number(value: number), [|Word|] }` }],
  },
  {
    code: "variant-invalid-field-type",
    examples: [
      {
        id: "notAType",
        title: "a field whose type does not parse",
        fixTitle: "a field type that parses",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export variant Shape { Circle(r: [|number =|]) }` },
        good: { "main.EXT": `export variant Shape { Circle(r: number) }` },
      },
    ],
    explain: [{ role: "error", units: `variant Reading { Sample(at: [|Date =|]) }` }],
  },
  {
    code: "variant-field-shadows-tag",
    examples: [
      {
        id: "kindField",
        title: "a payload field named `kind`",
        fixTitle: "the field renamed",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export variant Token { Word([|kind|]: string) }` },
        good: { "main.EXT": `export variant Token { Word(text: string) }` },
      },
    ],
    explain: [
      { role: "error", units: `variant Token { Word([|kind|]: string) }` },
      { role: "fix", units: `variant Token { Word(text: string) }` },
    ],
  },
  {
    code: "variant-duplicate-field",
    examples: [
      {
        id: "repeatedField",
        title: "a case declaring one field twice",
        fixTitle: "the second field renamed",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export variant Reading { Sample(at: number, [|at|]: string) }` },
        good: { "main.EXT": `export variant Reading { Sample(at: number, label: string) }` },
      },
    ],
    explain: [
      { role: "error", units: `variant Reading { Sample(at: Date, value: number, [|at|]: string) }` },
      { role: "fix", units: `variant Reading { Sample(at: Date, value: number, label: string) }` },
    ],
  },
  {
    code: "variant-required-after-optional",
    examples: [
      {
        id: "requiredLast",
        title: "a required field after an optional one",
        fixTitle: "the required field first",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export variant Request { Get(timeout?: number, [|url|]: string) }` },
        good: { "main.EXT": `export variant Request { Get(url: string, timeout?: number) }` },
      },
    ],
    explain: [
      { role: "error", units: `variant Request { Get(timeout?: number, [|url|]: string) }\nexport {};` },
      { role: "fix", units: `variant Request { Get(url: string, timeout?: number) }\nexport {};` },
    ],
  },
  {
    code: "variant-default-export",
    examples: [
      {
        id: "defaultExport",
        title: "a variant exported as the default",
        fixTitle: "the variant exported and imported by name",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export [|default|] variant Dir { Up, Down }` },
        good: {
          "shape.EXT": `export variant Dir { Up, Down }`,
          "main.EXT": `import { Dir } from "./shape.EXT";\nexport const up = Dir.Up;`,
        },
      },
    ],
    explain: [
      { role: "error", units: `export [|default|] variant Dir { Up, Down }` },
      {
        role: "fix",
        units: {
          "shape.EXT": `// shape.tt\nexport variant Dir { Up, Down }`,
          "use.EXT": `// use.tt\nimport { Dir } from "./shape.tt";\nexport const up = Dir.Up;`,
        },
      },
    ],
  },
  {
    code: "pattern-duplicate-binding",
    examples: [
      {
        ...numberMatch,
        id: "sameNameTwice",
        title: "a pattern binding one name from two fields",
        fixTitle: "the second field bound under its own name",
        decls: [shape],
        In: "Shape",
        inputs: shapes,
        bad: (x) => `match (${x}) { [|Rect|](w, h: w) => note("rect", w), Circle(r) => note("circle", r), Point => note("point", 0) }`,
        tt: (x) => `match (${x}) { Rect(w, h) => note("rect", w * h), Circle(r) => note("circle", r), Point => note("point", 0) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t}.kind === "Rect" ? note("rect", ${t}.w * ${t}.h) : ${t}.kind === "Circle" ? note("circle", ${t}.r) : note("point", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Rect(width: number, height: number), Point }\nexport function area(s: Shape) {\n  return (\n    match (s) {\n      [|Rect|](width: w, height: w) => w * w,\n      Point => 0,\n    }\n  );\n}`,
      },
      {
        role: "fix",
        units: `export variant Shape { Rect(width: number, height: number), Point }\nexport function area(s: Shape) {\n  return (\n    match (s) {\n      Point => 0,\n      Rect(width: w, height: h) => w * h\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-mixed-patterns",
    examples: [
      {
        ...numberMatch,
        id: "tagAndLiteral",
        title: "a match mixing a tag pattern with a literal pattern",
        fixTitle: "the literal arm removed",
        decls: [shape],
        In: "Shape",
        inputs: shapes,
        bad: (x) => `match (${x}) { Circle(r) => note("circle", r), [|1|] => note("one", 1), _ => note("other", 0) }`,
        tt: (x) => `match (${x}) { Circle(r) => note("circle", r), _ => note("other", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t}.kind === "Circle" ? note("circle", ${t}.r) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(r: number), Point }\nexport function size(s: Shape) {\n  return (\n    match (s) {\n      Circle(r) => r,\n      [|0|] => 0,\n      _ => -1,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-wildcard-not-last",
    examples: [
      {
        ...numberMatch,
        id: "wildcardFirst",
        title: "a wildcard arm before another arm",
        fixTitle: "the wildcard arm moved to the end",
        bad: (x) => `match (${x}) { [|_|] => note("other", 0), 1 => note("one", 1) }`,
        tt: (x) => `match (${x}) { 1 => note("one", 1), _ => note("other", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("one", 1) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function describe(code: number) {\n  return (\n    match (code) {\n      [|_|] => "other",\n      200 => "ok",\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-or-literal-kind-mismatch",
    examples: [
      {
        ...numberMatch,
        id: "numberOrString",
        title: "an or-pattern of a number and a string",
        fixTitle: "one arm per kind of literal",
        In: "(number | string)",
        inputs: '[1, "a", 2]',
        bad: (x) => `match (${x}) { 1 | [|"a"|] => note("hit", 0), _ => note("other", 1) }`,
        tt: (x) => `match (${x}) { 1 => note("hit", 0), "a" => note("hit", 0), _ => note("other", 1) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("hit", 0) : ${t} === "a" ? note("hit", 0) : note("other", 1))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function describe(key: string | number) {\n  return (\n    match (key) {\n      "a" | [|1|] => "first",\n      _ => "other",\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-duplicate-arm",
    examples: [
      {
        ...numberMatch,
        id: "hexRepeat",
        title: "a number arm repeated in hexadecimal",
        fixTitle: "the repeated arm removed",
        bad: (x) => `match (${x}) { 1 => note("one", 1), [|0x1|] => note("hex", 2), _ => note("other", 0) }`,
        tt: (x) => `match (${x}) { 1 => note("one", 1), _ => note("other", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("one", 1) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function describe(status: number) {\n  return (\n    match (status) {\n      200 => "ok",\n      [|0xc8|] => "also ok",\n      _ => "other",\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-is-wildcard-required",
    examples: [
      {
        ...numberMatch,
        id: "noWildcard",
        title: "an `is` match without a final wildcard",
        fixTitle: "a final wildcard arm",
        In: "unknown",
        inputs: '[new Error("boom"), new Date(0), 1]',
        bad: (x) => `[|match|] (${x}) { is Error => note("error", 0), is Date => note("date", 1) }`,
        tt: (x) => `match (${x}) { is Error => note("error", 0), is Date => note("date", 1), _ => note("other", 2) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t} instanceof Error ? note("error", 0) : ${t} instanceof Date ? note("date", 1) : note("other", 2))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function kind(value: unknown) {\n  return (\n    [|match|] (value) {\n      is Error => "error",\n      is Date => "date",\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-is-empty-bindings",
    examples: [
      {
        ...numberMatch,
        id: "emptyBraces",
        title: "an `is` pattern with an empty binding list",
        fixTitle: "the braces removed",
        In: "unknown",
        inputs: '[new Error("boom"), 1]',
        bad: (x) => `match (${x}) { [|is Error {}|] => note("error", 0), _ => note("other", 1) }`,
        tt: (x) => `match (${x}) { is Error => note("error", 0), _ => note("other", 1) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} instanceof Error ? note("error", 0) : note("other", 1))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function kind(value: unknown) {\n  return (\n    match (value) {\n      [|is Error {}|] => "error",\n      _ => "other",\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-is-or-bindings",
    examples: [
      {
        ...numberMatch,
        id: "bindingAlternatives",
        title: "an `is` or-pattern that binds a property",
        fixTitle: "one arm per class",
        In: "unknown",
        inputs: '[new TypeError("type"), new RangeError("range"), 1]',
        bad: (x) =>
          `match (${x}) { [|is TypeError { message }|] | is RangeError { message } => note("bad", message), _ => note("other", "") }`,
        tt: (x) => `match (${x}) { is TypeError { message } => note("bad", message), is RangeError { message } => note("bad", message), _ => note("other", "") }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t} instanceof TypeError ? note("bad", ${t}.message) : ${t} instanceof RangeError ? note("bad", ${t}.message) : note("other", ""))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function message(value: unknown) {\n  return (\n    match (value) {\n      [|is TypeError { message }|] | is RangeError { message } => message,\n      _ => "",\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-placement",
    examples: [
      {
        ...numberMatch,
        id: "noStatementPosition",
        title: "a match where its host has no statement position",
        fixTitle: "the match lifted into a declaration before the host",
        where: "rejects",
        jsx: false,
        bad: (x) => `[|match (${x}) { 1 => note("one", 1), _ => note("other", 0) }|]`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("one", 1) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `function scale(factor: number, size = [|match (factor) { 0 => 1, _ => factor }|]) {\n  return size;\n}\nexport { scale };`,
      },
    ],
  },
  {
    code: "match-control-crossing",
    examples: [
      {
        id: "breakOut",
        title: "a break in a match arm that leaves the loop around the match",
        fixTitle: "the break moved after the match",
        family: "match",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[1, 2]",
        temps: 1,
        result: "seen",
        bad: (x) =>
          `let seen: unknown = "none";\nfor (const round of [1, 2]) { const v = match (${x}) { 1 => { if (round > 1) [|break|]; return note("one", round); }, _ => note("other", round) }; seen = v; }`,
        tt: (x) =>
          `let seen: unknown = "none";\nfor (const round of [1, 2]) { const v = match (${x}) { 1 => note("one", round), _ => note("other", round) }; if (round > 1) break; seen = v; }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\nfor (const round of [1, 2]) { const ${t} = ${x}; const v = ${t} === 1 ? note("one", round) : note("other", round); if (round > 1) break; seen = v; }`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Item { Stop, Keep(value: number) }\nexport function sum(items: Item[]) {\n  let total = 0;\n  for (const item of items) {\n    total += match (item) {\n      Stop => { if (total > 9) [|break|]; return 0; },\n      Keep(value) => value,\n    };\n  }\n  return total;\n}`,
      },
    ],
  },
  {
    code: "match-nested-in-or-pattern",
    examples: [
      {
        ...numberMatch,
        id: "nestedAlternative",
        title: "an or-pattern with a nested alternative",
        fixTitle: "one arm per alternative",
        decls: [opt, outcome],
        In: "Outcome",
        inputs: outcomes,
        bad: (x) => `match (${x}) { [|Done|](value: None()) | Failed => note("hit", 0), _ => note("other", 1) }`,
        tt: (x) => `match (${x}) { Done(value: None()) => note("hit", 0), Failed => note("hit", 0), _ => note("other", 1) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t}.kind === "Done" && ${t}.value.kind === "None" ? note("hit", 0) : ${t}.kind === "Failed" ? note("hit", 0) : note("other", 1))`,
      },
      {
        ...numberMatch,
        id: "bindingAlternative",
        title: "an or-pattern whose nested alternative binds the name the other alternative binds",
        fixTitle: "one arm per alternative",
        only: ["declarationInitializer"],
        jsx: false,
        decls: [opt, outcome],
        In: "Outcome",
        inputs: outcomes,
        bad: (x) => `match (${x}) { [|Done|](value: Some(value: v)) | Failed(error: v) => note("hit", v), _ => note("other", 0) }`,
        tt: (x) => `match (${x}) { Done(value: Some(value: v)) => note("hit", v), Failed(error: v) => note("hit", v), _ => note("other", 0) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t}.kind === "Done" && ${t}.value.kind === "Some" ? note("hit", ${t}.value.value) : ${t}.kind === "Failed" ? note("hit", ${t}.error) : note("other", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Maybe { Some(value: number), None }\nexport variant Outcome { Ok(value: Maybe), Err(error: string) }\nexport function value(o: Outcome) {\n  return (\n    match (o) {\n      [|Ok|](value: None()) | Err => 0,\n      _ => -1,\n    }\n  );\n}`,
      },
      {
        role: "fix",
        units: `export variant Maybe { Some(value: number), None }\nexport variant Outcome { Ok(value: Maybe), Err(error: string) }\nexport function value(o: Outcome) {\n  return (\n    match (o) {\n      Ok(value: Some(value: v)) => v,\n      Ok(value: None()) => 0,\n      Err(error) => -1,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-or-binding-mismatch",
    examples: [
      {
        ...numberMatch,
        id: "differentNames",
        title: "or-pattern alternatives binding different names",
        fixTitle: "one arm per alternative",
        decls: [shape],
        In: "Shape",
        inputs: shapes,
        bad: (x) => `match (${x}) { Circle(r) | [|Rect|](w) => note("size", 0), Point => note("point", 0) }`,
        tt: (x) => `match (${x}) { Circle(r) => note("size", r), Rect(w) => note("size", w), Point => note("point", 0) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t}.kind === "Circle" ? note("size", ${t}.r) : ${t}.kind === "Rect" ? note("size", ${t}.w) : note("point", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(radius: number), Square(side: number) }\nexport function size(s: Shape) {\n  return (\n    match (s) {\n      Circle(radius) | [|Square|](side) => 0,\n    }\n  );\n}`,
      },
      {
        role: "fix",
        units: `export variant Shape { Circle(radius: number), Square(side: number) }\nexport function size(s: Shape) {\n  return (\n    match (s) {\n      Circle(radius) => radius,\n      Square(side) => side,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-tuple-arity",
    examples: [
      {
        ...numberMatch,
        id: "shortTuple",
        title: "a one-element tuple pattern in a two-scrutinee match",
        fixTitle: "the tuple pattern given both elements",
        decls: [dir],
        In: "Dir",
        inputs: "[Dir.Up, Dir.Down]",
        temps: 2,
        bad: (x) => `match (${x}, note<Dir>("second", Dir.Up)) { (Up, Down) => note("ud", 0), [|(Up)|] => note("u", 1), _ => note("other", 2) }`,
        tt: (x) => `match (${x}, note<Dir>("second", Dir.Up)) { (Up, Down) => note("ud", 0), (Up, _) => note("u", 1), _ => note("other", 2) }`,
        ts: (x, [t, u]) =>
          `(${t} = ${x}, ${u} = note<Dir>("second", Dir.Up), ${t}.kind === "Up" && ${u}.kind === "Down" ? note("ud", 0) : ${t}.kind === "Up" ? note("u", 1) : note("other", 2))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Dir { Up, Down }\nexport function both(a: Dir, b: Dir) {\n  return (\n    match (a, b) {\n      (Up, Up) => 2,\n      [|(Down)|] => 1,\n      _ => 0,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "unknown-case",
    examples: [
      {
        ...numberMatch,
        id: "misspelledTag",
        title: "a misspelled case tag",
        fixTitle: "the tag spelled as declared",
        decls: [shape],
        In: "Shape",
        inputs: shapes,
        bad: (x) => `match (${x}) { [|Circel|](r) => note("circle", r), Rect(w, h) => note("rect", w * h), Point => note("point", 0) }`,
        tt: (x) => `match (${x}) { Circle(r) => note("circle", r), Rect(w, h) => note("rect", w * h), Point => note("point", 0) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t}.kind === "Circle" ? note("circle", ${t}.r) : ${t}.kind === "Rect" ? note("rect", ${t}.w * ${t}.h) : note("point", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(r: number), Point }\nexport function size(s: Shape) {\n  return (\n    match (s) {\n      [|Circel|](r) => r,\n      Point => 0,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "unknown-field",
    examples: [
      {
        ...numberMatch,
        id: "misspelledField",
        title: "a misspelled payload field",
        fixTitle: "the field spelled as declared",
        decls: [disk],
        In: "Disk",
        inputs: disks,
        bad: (x) => `match (${x}) { Round([|radiu|]) => note("round", radiu), Dot => note("dot", 0) }`,
        tt: (x) => `match (${x}) { Round(radius) => note("round", radius), Dot => note("dot", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t}.kind === "Round" ? note("round", ${t}.radius) : note("dot", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(radius: number), Point }\nexport function size(s: Shape) {\n  return (\n    match (s) {\n      Circle([|radus|]) => radus,\n      Point => 0,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "match-not-exhaustive",
    examples: [
      {
        ...numberMatch,
        id: "missingCase",
        title: "a match missing one case",
        fixTitle: "the missing arm added",
        decls: [shape],
        In: "Shape",
        inputs: shapes,
        bad: (x) => `[|match (${x})|] { Circle(r) => note("circle", r), Rect(w, h) => note("rect", w * h) }`,
        tt: (x) => `match (${x}) { Circle(r) => note("circle", r), Rect(w, h) => note("rect", w * h), Point => note("point", 0) }`,
        ts: (x, [t]) =>
          `(${t} = ${x}, ${t}.kind === "Circle" ? note("circle", ${t}.r) : ${t}.kind === "Rect" ? note("rect", ${t}.w * ${t}.h) : note("point", 0))`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(r: number), Point }\nexport function size(s: Shape) {\n  return (\n    [|match (s)|] {\n      Circle(r) => r,\n    }\n  );\n}`,
      },
    ],
  },
  {
    code: "val-mutation",
    examples: [
      {
        id: "propertyWrite",
        title: "a property written through a val binding",
        fixTitle: "a copy with the new value",
        family: "val",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[3, 8]",
        result: "seen",
        bad: (x) => `val const cfg = { n: ${x} };\n[|cfg|].n = 2;\nconst seen = cfg.n;`,
        tt: (x) => `val const cfg = { n: ${x} };\nconst changed = { ...cfg, n: 2 };\nconst seen = [cfg.n, changed.n];`,
        ts: (x) => `const cfg = { n: ${x} };\nconst changed = { ...cfg, n: 2 };\nconst seen = [cfg.n, changed.n];`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `export function reset() {\n  val const settings = { retries: 3 };\n  [|settings|].retries = 0;\n  return settings;\n}`,
      },
    ],
  },
  {
    code: "val-pass",
    examples: [
      {
        id: "mutableParameter",
        title: "a val binding passed to a mutable parameter",
        fixTitle: "the parameter declared `val`",
        family: "val",
        kind: "statement",
        jsx: true,
        In: "number",
        inputs: "[3, 8]",
        result: "seen",
        bad: (x) => `function take(p: { n: number }) {\nreturn note("take", p.n);\n}\nval const cfg = { n: ${x} };\nconst seen = take([|cfg|]);`,
        tt: (x) => `function take(val p: { n: number }) {\nreturn note("take", p.n);\n}\nval const cfg = { n: ${x} };\nconst seen = take(cfg);`,
        ts: (x) => `function take(p: { n: number }) {\nreturn note("take", p.n);\n}\nconst cfg = { n: ${x} };\nconst seen = take(cfg);`,
      },
    ],
    explain: [
      {
        role: "error",
        units: `function bump(counter: { n: number }) {\n  counter.n++;\n}\nexport function run() {\n  val const counter = { n: 0 };\n  bump([|counter|]);\n}`,
      },
    ],
  },
  {
    code: "verify-failed",
    examples: [
      {
        id: "missingExpression",
        title: "a file with no tt construct whose TypeScript does not parse",
        fixTitle: "the initializer written",
        kind: "module",
        surfaces: ["tt", "ttx"],
        bad: { "main.EXT": `export const limit = [|;|]` },
        good: { "main.EXT": `export const limit = 10;` },
      },
    ],
    explain: [{ role: "error", units: `export const limit = [|;|]` }],
  },
  {
    code: "source-not-typescript",
    examples: [
      {
        ...numberMatch,
        id: "armBody",
        title: "a match arm body that is not TypeScript",
        skip: ["optionalCall", "topLevel"],
        fixTitle: "the arm body completed",
        bad: (x) => `match (${x}) { 1 => note("one", 1) +[|,|] _ => note("other", 0) }`,
        tt: (x) => `match (${x}) { 1 => note("one", 1) + 1, _ => note("other", 0) }`,
        ts: (x, [t]) => `(${t} = ${x}, ${t} === 1 ? note("one", 1) + 1 : note("other", 0))`,
      },
      {
        id: "resultBody",
        title: "a result block body that is not TypeScript",
        skip: ["optionalCall", "topLevel"],
        fixTitle: "the expression completed",
        family: "result",
        kind: "value",
        jsx: true,
        In: "number",
        inputs: "[2, -1]",
        bad: (x) => `result { const n = try read(${x}) *[|;|] return n; }`,
        tt: (x) => `result { const n = try read(${x}) * 2; return n; }`,
        ts: result((x) => `const n = unwrap(read(${x})) * 2; return ok(n);`),
      },
      {
        id: "resultBodyEnd",
        title: "a result block whose last statement is an expression the block's `}` cuts off",
        fixTitle: "the expression completed",
        family: "result",
        kind: "value",
        only: ["declarationInitializer"],
        In: "number",
        inputs: "[2, -1]",
        bad: (x) => `result { return try read(${x}) + [|}|]`,
        tt: (x) => `result { return try read(${x}) + 1 }`,
        ts: result((x) => `return ok(unwrap(read(${x})) + 1);`),
      },
    ],
    explain: [
      {
        role: "error",
        units: `export variant Shape { Circle(r: number), Point }\nexport function size(s: Shape) {\n  return (\n    match (s) {\n      Circle(r) => r *[|,|]\n      Point => 0,\n    }\n  );\n}`,
      },
    ],
  },
];
