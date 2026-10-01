//// [aValueNestedInAForHeadInitializerRunsBeforeTheLoop4.tt] ////
variant O { A(n: number), B }
declare const o: O;
declare function g(n: number): number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export function f(): R { for (let x = try r(), i = 0; i < 1; i++) {} return r(); }


//// [aValueNestedInAForHeadInitializerRunsBeforeTheLoop4.ts]
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function g(n: number): number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export function f(): R { let $tt_v0: number;
const $tt_t0 = r();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
for (let x = $tt_v0, i = 0; i < 1; i++) {} return r(); }
