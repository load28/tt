import { variant } from "./variants.mjs";

const token = variant("Token", [
  ["Num", [["value", "number"]]],
  ["Neg", [["value", "number"]]],
  ["Word", [["text", "string"]]],
]);
const maybe = variant("Maybe", [
  ["Some", [["value", "number"]]],
  ["None", null],
]);
const outcome = variant("Outcome", [
  ["Done", [["value", "Maybe"]]],
  ["Failed", [["error", "string"]]],
]);

export default [
  {
    construct: "ifLet",
    kind: "statement",
    jsx: true,
    forms: [
      {
        id: "thenOnly",
        title: "a body without an else",
        In: "number",
        inputs: "[2, -1]",
        tt: (x) => `let seen: unknown = "none";\nif let Ok(value: v) = read(${x}) { seen = note("then", v); }`,
        ts: (x, [t]) => `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const v = ${t}.value; seen = note("then", v); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "withElse",
        title: "a body and an else block",
        In: "number",
        inputs: "[3, -3]",
        tt: (x) => `let seen: unknown = "none";\nif let Ok(value: v) = read(${x}) { seen = note("then", v); } else { seen = note("else", "missing"); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const v = ${t}.value; seen = note("then", v); } else { seen = note("else", "missing"); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "elseIfLetChain",
        title: "an else-if-let chain ending in an else",
        In: "number",
        inputs: "[4, -4]",
        tt: (x) =>
          `let seen: unknown = "none";\nif let Ok(value: v) = read(${x}) { seen = note("first", v); } else if let Err(error) = read(note("second", -2)) { seen = note("second", error); } else { seen = "neither"; }`,
        ts: (x, [t, u]) =>
          `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const v = ${t}.value; seen = note("first", v); } else { const ${u} = read(note("second", -2)); if (${u}.kind === "Err") { const error = ${u}.error; seen = note("second", error); } else { seen = "neither"; } } }`,
        result: "seen",
        temps: 2,
      },
      {
        id: "nestedPattern",
        title: "a nested pattern",
        decls: [maybe, outcome],
        In: "Outcome",
        inputs: '[Outcome.Done(Maybe.Some(5)), Outcome.Done(Maybe.None), Outcome.Failed("gone")]',
        tt: (x) => `let seen: unknown = "none";\nif let Done(value: Some(value: v)) = ${x} { seen = note("some", v); } else { seen = "other"; }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = ${x};\nif (${t}.kind === "Done" && ${t}.value.kind === "Some") { const v = ${t}.value.value; seen = note("some", v); } else { seen = "other"; } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "orPattern",
        title: "an or-pattern",
        decls: [token],
        In: "Token",
        inputs: '[Token.Num(1), Token.Neg(-1), Token.Word("w")]',
        tt: (x) => `let seen: unknown = "none";\nif let Num(value) | Neg(value) = ${x} { seen = note("number", value); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = ${x};\nif (${t}.kind === "Num" || ${t}.kind === "Neg") { const value = ${t}.value; seen = note("number", value); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "objectLiteralHead",
        title: "an object literal as the bound expression",
        In: "number",
        inputs: "[6, 9]",
        tt: (x) => `let seen: unknown = "none";\nif let Some(value: v) = { kind: "Some" as const, value: ${x} } { seen = note("some", v); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = { kind: "Some" as const, value: ${x} };\nif (${t}.kind === "Some") { const v = ${t}.value; seen = note("some", v); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "exitInBody",
        title: "a body that leaves the host",
        In: "number",
        inputs: "[2, -2]",
        tt: (x, t, exit) => `if let Err(error) = read(${x}) { note("error", error); ${exit} }`,
        ts: (x, [t], exit) => `{ const ${t} = read(${x});\nif (${t}.kind === "Err") { const error = ${t}.error; note("error", error); ${exit} } }`,
        result: '"passed"',
        hoisted: '"passed"',
        temps: 1,
      },
    ],
  },
  {
    construct: "ifLetValue",
    kind: "value",
    only: ["declarationInitializer", "callArgument", "returnOperand", "arrowBody", "conditionalBranch", "templateLiteral"],
    rejects: { all: "if-let-placement" },
    forms: [
      {
        id: "asValue",
        title: "an if-let written where a value is expected",
        In: "number",
        inputs: "[1]",
        tt: (x) => `if let Ok(value: v) = read(${x}) { v }`,
        ts: () => "undefined",
      },
    ],
  },
];
