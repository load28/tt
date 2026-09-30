export default {
  construct: "result",
  kind: "value",
  jsx: true,
  rejects: { yield: "result-yield-crossing" },
  forms: [
    {
      id: "single",
      title: "one try and a returned value",
      In: "number",
      inputs: "[2, -1]",
      tt: (x) => `result { const n = try read(${x}); return n * 10; }`,
      ts: (x, t, c) => c.iife(`try { const n = unwrap(read(${x})); return ok(n * 10); } catch (error) { return caught(error); }`),
    },
    {
      id: "twoTries",
      title: "two tries, the second reading the first",
      In: "number",
      inputs: "[5, 1, -2]",
      tt: (x) => `result { const a = try read(${x}); const b = try read(note("b", a - 3)); return [a, b]; }`,
      ts: (x, t, c) =>
        c.iife(`try { const a = unwrap(read(${x})); const b = unwrap(read(note("b", a - 3))); return ok([a, b]); } catch (error) { return caught(error); }`),
    },
    {
      id: "earlyReturn",
      title: "a return inside an if that completes the block early",
      In: "number",
      inputs: "[3, 1, -1]",
      tt: (x) => `result { const n = try read(${x}); if (n > 2) { return note("big", n); } return note("small", n); }`,
      ts: (x, t, c) =>
        c.iife(`try { const n = unwrap(read(${x})); if (n > 2) { return ok(note("big", n)); } return ok(note("small", n)); } catch (error) { return caught(error); }`),
    },
    {
      id: "bareReturn",
      title: "a bare return that completes the block with Ok(undefined)",
      In: "number",
      inputs: "[2, -1]",
      tt: (x) => `result { try read(${x}); return; }`,
      ts: (x, t, c) => c.iife(`try { unwrap(read(${x})); return ok(undefined); } catch (error) { return caught(error); }`),
    },
    {
      id: "nested",
      title: "a nested result block whose failure the outer one reads",
      In: "number",
      inputs: "[2, -1]",
      tt: (x) =>
        `result { const inner = result { const n = try read(${x}); return n + 1; }; note("inner", inner); const m = try inner; return m * 2; }`,
      ts: (x, t, c) =>
        c.iife(
          `try { const inner = ${c.iife(`try { const n = unwrap(read(${x})); return ok(n + 1); } catch (error) { return caught(error); }`)}; note("inner", inner); const m = unwrap(inner); return ok(m * 2); } catch (error) { return caught(error); }`,
        ),
    },
    {
      id: "returnsMatch",
      title: "a returned match over a value the block unwrapped",
      In: "number",
      inputs: "[2, 7, -1]",
      tt: (x) => `result { const n = try read(${x}); return match (n) { 2 => "two", _ => "other" }; }`,
      ts: (x, [t], c) =>
        c.iife(`try { const n = unwrap(read(${x})); return ok((${t} = n, ${t} === 2 ? "two" : "other")); } catch (error) { return caught(error); }`),
    },
    {
      id: "ifLetInside",
      title: "an if-let whose body completes the block",
      In: "number",
      inputs: "[4, -1]",
      tt: (x) =>
        `result { if let Ok(value: n) = read(${x}) { return note("direct", n); } const m = try read(note("fallback", -7)); return m; }`,
      ts: (x, t, c) =>
        c.iife(
          `try { const head = read(${x}); if (head.kind === "Ok") { const n = head.value; return ok(note("direct", n)); } const m = unwrap(read(note("fallback", -7))); return ok(m); } catch (error) { return caught(error); }`,
        ),
    },
  ],
};
