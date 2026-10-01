//// [aReturnedTemplateLiteralThatEndsItsStatementKeepsTheReturnSuffix4.tt] ////
variant O { A(n: number), B }
declare const o: O;
declare function tag(s: TemplateStringsArray, ...v: unknown[]): string;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export const a = result { const x = try r(); if (x > 0) return `a`
  return `b`};


//// [aReturnedTemplateLiteralThatEndsItsStatementKeepsTheReturnSuffix4.ts]
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function tag(s: TemplateStringsArray, ...v: unknown[]): string;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
let $tt_v0: ({
    kind: "Err";
    error: string;
}) | ({
    kind: "Ok";
    value: string;
});
$tt_v0: {
  const $tt_t0 = r();
  if (!("value" in $tt_t0)) {
    $tt_v0 = $tt_t0;
    break $tt_v0;
  }
  const x = $tt_t0.value; if (x > 0) { const $tt_a0 = { value: { kind: "Ok" as const, value: `a` } }; $tt_v0 = $tt_a0.value; break $tt_v0; }
  {
    const $tt_a1 = { value: { kind: "Ok" as const, value: `b` } };
    $tt_v0 = $tt_a1.value;
    break $tt_v0;
  }
}
export const a = $tt_v0;
