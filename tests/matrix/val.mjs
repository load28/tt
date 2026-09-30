const strip = (text) => text.replace(/\bval /g, "");

const diagnostic = ["functionBody", "nestedBlock", "arrowBlock", "method", "matchBlockArm", "resultBody", "topLevel", "componentBody"];

const forms = [
  {
    id: "constObject",
    title: "a val const read through its paths",
    tt: (x) => `val const cfg = { n: ${x}, inner: { m: 1 } };\nconst seen = note("read", cfg.n + cfg.inner.m);`,
  },
  {
    id: "letRebound",
    title: "a val let rebound to a new object",
    tt: (x) => `val let state = { n: ${x} };\nstate = { n: state.n * 2 };\nconst seen = state.n;`,
  },
  {
    id: "parameter",
    title: "a val parameter of a function declaration",
    tt: (x) => `function readN(val p: { n: number }) {\nreturn note("param", p.n);\n}\nconst seen = readN({ n: ${x} });`,
  },
  {
    id: "passedToValParameter",
    title: "a val binding passed to a val parameter",
    tt: (x) => `const show = (val p: { n: number }) => note("show", p.n);\nval const cfg = { n: ${x} };\nconst seen = show(cfg);`,
  },
  {
    id: "forOfBinding",
    title: "a val for-of binding",
    tt: (x) => `let seen = 0;\nfor (val const item of [{ n: ${x} }, { n: 1 }]) {\nseen += note("item", item.n);\n}`,
  },
  {
    id: "catchBinding",
    title: "a val catch binding",
    tt: (x) => `let seen: unknown = null;\ntry {\nthrow { n: ${x} };\n} catch (val e) {\nseen = note("caught", e);\n}`,
  },
  {
    id: "spreadCopy",
    title: "a copy spread from a val binding and then mutated",
    tt: (x) => `val const cfg = { n: ${x} };\nconst copy = { ...cfg };\ncopy.n = note("copy", copy.n + 1);\nconst seen = [cfg.n, copy.n];`,
  },
  {
    id: "originalMutated",
    title: "a val alias whose original is mutated",
    tt: (x) => `const original = { n: ${x} };\nval const view = original;\noriginal.n = note("original", original.n + 10);\nconst seen = view.n;`,
  },
  {
    id: "propertyWrite",
    title: "a property written through a val binding",
    only: diagnostic,
    rejects: { all: "val-mutation" },
    tt: (x) => `val const cfg = { n: ${x} };\ncfg.n = 2;\nconst seen = cfg.n;`,
  },
  {
    id: "deepIncrement",
    title: "a nested property incremented through a val binding",
    only: diagnostic,
    rejects: { all: "val-mutation" },
    tt: (x) => `val const cfg = { a: { n: ${x} } };\ncfg.a.n++;\nconst seen = cfg.a.n;`,
  },
  {
    id: "passedToMutableParameter",
    title: "a val binding passed to a mutable parameter",
    only: diagnostic,
    rejects: { all: "val-pass" },
    tt: (x) => `function bump(p: { n: number }) {\np.n++;\n}\nval const cfg = { n: ${x} };\nbump(cfg);\nconst seen = cfg.n;`,
  },
  {
    id: "builtinMutator",
    title: "a built-in mutating method called through a val binding",
    only: diagnostic,
    rejects: { all: "val-mutation" },
    tt: (x) => `val const items: number[] = [];\nitems.push(${x});\nconst seen = items;`,
  },
];

export default {
  construct: "val",
  kind: "statement",
  jsx: true,
  forms: forms.map((form) => ({
    In: "number",
    inputs: "[3, 8]",
    result: "seen",
    ...form,
    ts: (x, t, exit) => strip(form.tt(x, t, exit)),
  })),
};
