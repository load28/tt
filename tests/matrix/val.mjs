const strip = (text) => text.replace(/\bval /g, "");

const diagnostic = ["functionBody", "nestedBlock", "arrowBlock", "method", "matchBlockArm", "resultBody", "topLevel", "componentBody"];

const forms = [
  {
    id: "constObject",
    title: "a val const read through its paths",
    tt: (x) => `val const /*@bind*/cfg = { n: ${x}, inner: { m: 1 } };\nconst seen = note("read", /*@use*/cfg./*@member*/n + cfg.inner.m);`,
  },
  {
    id: "letRebound",
    title: "a val let rebound to a new object",
    tt: (x) => `val let /*@bind*/state = { n: ${x} };\nstate = { n: /*@use*/state./*@member*/n * 2 };\nconst seen = state.n;`,
  },
  {
    id: "parameter",
    title: "a val parameter of a function declaration",
    tt: (x) =>
      `function readN(val /*@bind*/p: { n: number }) {\nreturn note("param", /*@use*/p./*@member*/n);\n}\nconst seen = /*@call*/readN(/*@arg*/{ n: ${x} });`,
  },
  {
    id: "passedToValParameter",
    title: "a val binding passed to a val parameter",
    tt: (x) =>
      `const show = (val /*@bind*/p: { n: number }) => note("show", /*@use*/p.n);\nval const /*@bind2*/cfg = { n: ${x} };\nconst seen = /*@call*/show(/*@use2*/cfg);`,
  },
  {
    id: "forOfBinding",
    title: "a val for-of binding",
    tt: (x) => `let seen = 0;\nfor (val const /*@bind*/item of [{ n: ${x} }, { n: 1 }]) {\nseen += note("item", /*@use*/item./*@member*/n);\n}`,
  },
  {
    id: "catchBinding",
    title: "a val catch binding",
    tt: (x) => `let seen: unknown = null;\ntry {\nthrow { n: ${x} };\n} catch (val /*@bind*/e) {\nseen = note("caught", /*@use*/e);\n}`,
  },
  {
    id: "spreadCopy",
    title: "a copy spread from a val binding and then mutated",
    tt: (x) =>
      `val const /*@bind*/cfg = { n: ${x} };\nconst /*@bind2*/copy = { .../*@use*/cfg };\ncopy.n = note("copy", /*@use2*/copy./*@member*/n + 1);\nconst seen = [cfg.n, copy.n];`,
  },
  {
    id: "originalMutated",
    title: "a val alias whose original is mutated",
    tt: (x) =>
      `const /*@bind2*/original = { n: ${x} };\nval const /*@bind*/view = /*@use2*/original;\noriginal.n = note("original", original.n + 10);\nconst seen = /*@use*/view./*@member*/n;`,
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
