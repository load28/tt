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
        tt: (x) => `let seen: unknown = "none";\nif let Ok(/*@field*/value: /*@bind*/v) = /*@call*/read(/*@arg*/${x}) { seen = note("then", /*@use*/v); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = /*@call*/read(/*@arg*/${x});\nif (${t}.kind === "Ok") { const /*@bind*/v = ${t}./*@field*/value; seen = note("then", /*@use*/v); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "withElse",
        title: "a body and an else block",
        In: "number",
        inputs: "[3, -3]",
        tt: (x) =>
          `let seen: unknown = "none";\nif let Ok(value: /*@bind*/v) = read(${x}) { seen = note("then", /*@use*/v); } else { seen = /*@call*/note(/*@arg*/"else", "missing"); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const /*@bind*/v = ${t}.value; seen = note("then", /*@use*/v); } else { seen = /*@call*/note(/*@arg*/"else", "missing"); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "elseIfLetChain",
        title: "an else-if-let chain ending in an else",
        In: "number",
        inputs: "[4, -4]",
        tt: (x) =>
          `let seen: unknown = "none";\nif let Ok(value: /*@bind*/v) = read(${x}) { seen = note("first", /*@use*/v); } else if let Err(/*@bind2*/error) = /*@call*/read(/*@arg*/note("second", -2)) { seen = note("second", /*@use2*/error); } else { seen = "neither"; }`,
        ts: (x, [t, u]) =>
          `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const v = ${t}.value; seen = note("first", v); } else { const ${u} = read(note("second", -2)); if (${u}.kind === "Err") { const error = ${u}.error; seen = note("second", error); } else { seen = "neither"; } } }`,
        edit: (x, [t, u]) =>
          `let seen: unknown = "none";\n{ const ${t} = read(${x});\nif (${t}.kind === "Ok") { const /*@bind*/v = ${t}.value; seen = note("first", /*@use*/v); } else { const ${u} = /*@call*/read(/*@arg*/note("second", -2)); if (${u}.kind === "Err") { const { /*@bind2*/error } = ${u}; seen = note("second", /*@use2*/error); } else { seen = "neither"; } } }`,
        result: "seen",
        temps: 2,
      },
      {
        id: "nestedPattern",
        title: "a nested pattern",
        decls: [maybe, outcome],
        In: "Outcome",
        inputs: '[Outcome.Done(Maybe.Some(5)), Outcome.Done(Maybe.None), Outcome.Failed("gone")]',
        tt: (x) => `let seen: unknown = "none";\nif let Done(value: Some(value: /*@bind*/v)) = ${x} { seen = note("some", /*@use*/v); } else { seen = "other"; }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = ${x};\nif (${t}.kind === "Done" && ${t}.value.kind === "Some") { const /*@bind*/v = ${t}.value.value; seen = note("some", /*@use*/v); } else { seen = "other"; } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "orPattern",
        title: "an or-pattern",
        decls: [token],
        In: "Token",
        inputs: '[Token.Num(1), Token.Neg(-1), Token.Word("w")]',
        tt: (x) => `let seen: unknown = "none";\nif let Num(/*@bind*/value) | Neg(value) = ${x} { seen = note("number", /*@use*/value); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = ${x};\nif (${t}.kind === "Num" || ${t}.kind === "Neg") { const value = ${t}.value; seen = note("number", value); } }`,
        edit: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = ${x};\nif (${t}.kind === "Num" || ${t}.kind === "Neg") { const { /*@bind*/value } = ${t}; seen = note("number", /*@use*/value); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "objectLiteralHead",
        title: "an object literal as the bound expression",
        In: "number",
        inputs: "[6, 9]",
        tt: (x) => `let seen: unknown = "none";\nif let Some(value: /*@bind*/v) = { kind: "Some" as const, value: ${x} } { seen = note("some", /*@use*/v); }`,
        ts: (x, [t]) =>
          `let seen: unknown = "none";\n{ const ${t} = { kind: "Some" as const, value: ${x} };\nif (${t}.kind === "Some") { const /*@bind*/v = ${t}.value; seen = note("some", /*@use*/v); } }`,
        result: "seen",
        temps: 1,
      },
      {
        id: "exitInBody",
        title: "a body that leaves the host",
        In: "number",
        inputs: "[2, -2]",
        tt: (x, t, exit) => `if let Err(/*@bind*/error) = read(${x}) { note("error", /*@use*/error); ${exit} }`,
        ts: (x, [t], exit) => `{ const ${t} = read(${x});\nif (${t}.kind === "Err") { const error = ${t}.error; note("error", error); ${exit} } }`,
        edit: (x, [t], exit) => `{ const ${t} = read(${x});\nif (${t}.kind === "Err") { const { /*@bind*/error } = ${t}; note("error", /*@use*/error); ${exit} } }`,
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
