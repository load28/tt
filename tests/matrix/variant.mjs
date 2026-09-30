import { variant } from "./variants.mjs";

const declared = (decl, use, extra = {}) => ({
  tt: (x) => `${decl.tt}\nconst seen = ${use(x)};`,
  ts: (x) => `${decl.ts}\nconst seen = ${use(x)};`,
  ...extra,
});

const diagnostic = ["functionBody", "nestedBlock", "arrowBlock", "matchBlockArm", "topLevel", "componentBody"];

const dir = variant("Dir", [["North", null], ["South", null]]);
const shape = variant("Shape", [
  ["Circle", [["r", "number"]]],
  ["Rect", [["w", "number"], ["h", "number"]]],
]);
const http = variant("Http", [
  ["Get", [["url", "string"], ["timeout", "number", true]]],
  ["Head", null],
]);
const tick = variant("Tick", [
  ["Now", []],
  ["Later", [["at", "number"]]],
]);
const tree = variant(
  "Tree",
  [
    ["Leaf", [["value", "T"]]],
    ["Node", [["left", "Tree<T>"], ["right", "Tree<T>"]]],
  ],
  { generics: "<T>" },
);
const meta = variant("Meta", [["kind", [["value", "number"]]], ["other", null]]);
const op = variant("Op", [["Add", [["n", "number"]]], ["Neg", null]]);
const exported = variant("Pub", [["Item", [["n", "number"]]]], { exported: true });

export default {
  construct: "variant",
  kind: "statement",
  jsx: true,
  forms: [
    {
      id: "unitCases",
      title: "unit cases, which are values",
      ...declared(dir, (x) => `[Dir.North, Dir.South, note("operand", ${x})]`),
    },
    {
      id: "payloadCases",
      title: "cases whose constructors take their fields in order",
      ...declared(shape, (x) => `[Shape.Circle(${x}), Shape.Rect(${x}, 2)]`),
    },
    {
      id: "optionalField",
      title: "an optional field set only when its argument is not undefined",
      ...declared(http, (x) => `[Http.Get("a"), Http.Get("b", ${x}), Http.Get("c", undefined), Http.Head]`),
    },
    {
      id: "emptyParentheses",
      title: "a case with empty parentheses, which is a function",
      ...declared(tick, (x) => `[Tick.Now(), Tick.Later(${x})]`),
    },
    {
      id: "generic",
      title: "a generic recursive variant",
      surfaces: ["tt"],
      ...declared(tree, (x) => `Tree.Node(Tree.Leaf(${x}), Tree.Node(Tree.Leaf(${x} + 1), Tree.Leaf(0)))`),
    },
    {
      id: "kindTag",
      title: "a case tagged `kind`",
      ...declared(meta, (x) => `[Meta.kind(${x}), Meta.other]`),
    },
    {
      id: "matchedWhereDeclared",
      title: "a variant matched where it is declared",
      tt: (x) => `${op.tt}\nconst seen = [Op.Add(${x}), Op.Neg].map((o) => match (o) { Add(n) => note("add", n + 1), Neg => note("neg", 0) });`,
      ts: (x, [t]) =>
        `${op.ts}\nconst seen = [Op.Add(${x}), Op.Neg].map((o) => { const ${t} = o; return ${t}.kind === "Add" ? note("add", ${t}.n + 1) : note("neg", 0); });`,
    },
    {
      id: "exported",
      title: "an exported variant",
      only: ["topLevel"],
      ...declared(exported, (x) => `[Pub.Item(${x})]`),
    },
    {
      id: "fieldShadowsTag",
      title: "a payload field named `kind`",
      only: diagnostic,
      rejects: { all: "variant-field-shadows-tag" },
      tt: (x) => `variant Bad { A(kind: string) }\nconst seen = [Bad.A("k"), ${x}];`,
    },
    {
      id: "requiredAfterOptional",
      title: "a required field after an optional one",
      only: diagnostic,
      rejects: { all: "variant-required-after-optional" },
      tt: (x) => `variant Bad { A(a?: number, b: number) }\nconst seen = [Bad.A(1, 2), ${x}];`,
    },
    {
      id: "duplicateCase",
      title: "a duplicate case tag",
      only: diagnostic,
      rejects: { all: "variant-duplicate-case" },
      tt: (x) => `variant Bad { A, B, A }\nconst seen = [Bad.A, ${x}];`,
    },
    {
      id: "defaultExport",
      title: "a default-exported variant",
      only: ["topLevel"],
      rejects: { all: "variant-default-export" },
      tt: (x) => `export default variant Bad { A }\nconst seen = [${x}];`,
    },
  ].map((form) => ({ In: "number", inputs: "[4, 9]", result: "seen", temps: 1, ...form })),
};
