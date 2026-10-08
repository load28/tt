//// [aLineCommentAtTheEndOfAMatchKeepsTheStatementEnd.tt] ////
variant S { A(n: number), B }
const s: S = S.A(3) as S;
function f() {
  // @ts-ignore
  const m = match (s) { A(n) => n, B => 4 // c
  };
  return m;
}
function g() {
  // @ts-ignore
  const k = match (s // c
  ) { A(n) => n + 1, B => 4 };
  return k;
}
console.log(f(), g());


//// [aLineCommentAtTheEndOfAMatchKeepsTheStatementEnd.ts]
var $tt_show: (value: unknown) => string = function (value) {
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (typeof value === "bigint") {
    return String(value) + "n";
  }
  if (typeof value === "object" || typeof value === "function") {
    try {
      const text = JSON.stringify(value);
      if (typeof text === "string") {
        return text;
      }
    } catch {}
    return typeof value;
  }
  return String(value);
};
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const s: S = S.A(3) as S;
function f() {
  // @ts-ignore
  let $tt_v0: number; { const $tt_m = s; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v0 = n; break; } case "B": { $tt_v0 = 4 // c
        ; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const m = $tt_v0;
  return m;
}
function g() {
  // @ts-ignore
  let $tt_v1: number; { const $tt_m = s // c
  ; switch ($tt_m.kind) {
  case "A": { const { n } = $tt_m; $tt_v1 = n + 1; break; } case "B": { $tt_v1 = 4; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const k = $tt_v1;
  return k;
}
console.log(f(), g());
