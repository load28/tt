//// [aValueNestedInAForHeadInitializerRunsBeforeTheLoop1.tt] ////
variant O { A(n: number), B }
declare const o: O;
declare function g(n: number): number;
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function r(): R;
export function f(): R { for (let x = g(match (o) { A(n) => n, B => 0 }); x < 1; x++) {} return r(); }


//// [aValueNestedInAForHeadInitializerRunsBeforeTheLoop1.ts]
function $tt_show(value: unknown): string {
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
}
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
const $tt_v1 = (g);
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v0 = $tt_v1(n);
      break;
    }
    case "B": {
      $tt_v0 = $tt_v1(0);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
for (let x = $tt_v0; x < 1; x++) {} return r(); }
