export default {
  construct: "result",
  kind: "value",
  jsx: true,
  rejects: { yield: "result-yield-crossing" },
  editorWithholds: { attrHost: ["hover"] },
  forms: [
    {
      id: "single",
      title: "one try and a returned value",
      In: "number",
      inputs: "[2, -1]",
      tt: (x) => `result { const /*@bind*/n = try /*@call*/read(${x}); return /*@use*/n * 10; }`,
      ts: (x, t, c) => c.iife(`try { const /*@bind*/n = unwrap(/*@call*/read(${x})); return ok(/*@use*/n * 10); } catch (error) { return caught(error); }`),
    },
    {
      id: "twoTries",
      title: "two tries, the second reading the first",
      In: "number",
      inputs: "[5, 1, -2]",
      tt: (x) =>
        `result { const /*@bind*/a = try read(${x}); const /*@bind2*/b = try read(/*@arg*/note("b", /*@use*/a - 3)); return [a, /*@use2*/b]; }`,
      ts: (x, t, c) =>
        c.iife(
          `try { const /*@bind*/a = unwrap(read(${x})); const /*@bind2*/b = unwrap(read(/*@arg*/note("b", /*@use*/a - 3))); return ok([a, /*@use2*/b]); } catch (error) { return caught(error); }`,
        ),
    },
    {
      id: "earlyReturn",
      title: "a return inside an if that completes the block early",
      In: "number",
      inputs: "[3, 1, -1]",
      tt: (x) => `result { const /*@bind*/n = try read(${x}); if (/*@use*/n > 2) { return note("big", n); } return note("small", n); }`,
      ts: (x, t, c) =>
        c.iife(
          `try { const /*@bind*/n = unwrap(read(${x})); if (/*@use*/n > 2) { return ok(note("big", n)); } return ok(note("small", n)); } catch (error) { return caught(error); }`,
        ),
    },
    {
      id: "bareReturn",
      title: "a bare return that completes the block with Ok(undefined)",
      In: "number",
      inputs: "[2, -1]",
      tt: (x) => `result { try /*@call*/read(/*@arg*/${x}); return; }`,
      ts: (x, t, c) => c.iife(`try { unwrap(/*@call*/read(/*@arg*/${x})); return ok(undefined); } catch (error) { return caught(error); }`),
    },
    {
      id: "nested",
      title: "a nested result block whose failure the outer one reads",
      In: "number",
      inputs: "[2, -1]",
      tt: (x) =>
        `result { const inner = result { const /*@bind*/n = try read(${x}); return /*@use*/n + 1; }; note("inner", inner); const m = try inner; return m * 2; }`,
      ts: (x, t, c) =>
        c.iife(
          `try { const inner = ${c.iife(`try { const /*@bind*/n = unwrap(read(${x})); return ok(/*@use*/n + 1); } catch (error) { return caught(error); }`)}; note("inner", inner); const m = unwrap(inner); return ok(m * 2); } catch (error) { return caught(error); }`,
        ),
      edit: (x, t, c) =>
        c.iife(
          `try { const inner: ReturnType<typeof read> = ${c.iife(`try { const /*@bind*/n = unwrap(read(${x})); return ok(/*@use*/n + 1); } catch (error) { return caught(error); }`)}; note("inner", inner); const m = unwrap(inner); return ok(m * 2); } catch (error) { return caught(error); }`,
        ),
    },
    {
      id: "returnsMatch",
      title: "a returned match over a value the block unwrapped",
      In: "number",
      inputs: "[2, 7, -1]",
      tt: (x) => `result { const /*@bind*/n = try read(${x}); return match (/*@use*/n) { 2 => "two", _ => "other" }; }`,
      ts: (x, [t], c) =>
        c.iife(`try { const /*@bind*/n = unwrap(read(${x})); return ok((${t} = /*@use*/n, ${t} === 2 ? "two" : "other")); } catch (error) { return caught(error); }`),
    },
    {
      id: "ifLetInside",
      title: "an if-let whose body completes the block",
      In: "number",
      inputs: "[4, -1]",
      tt: (x) =>
        `result { if let Ok(value: /*@bind*/n) = /*@call*/read(${x}) { return note("direct", /*@use*/n); } const /*@bind2*/m = try read(note("fallback", -7)); return /*@use2*/m; }`,
      ts: (x, t, c) =>
        c.iife(
          `try { const head = /*@call*/read(${x}); if (head.kind === "Ok") { const /*@bind*/n = head.value; return ok(note("direct", /*@use*/n)); } const /*@bind2*/m = unwrap(read(note("fallback", -7))); return ok(/*@use2*/m); } catch (error) { return caught(error); }`,
        ),
    },
  ],
};
