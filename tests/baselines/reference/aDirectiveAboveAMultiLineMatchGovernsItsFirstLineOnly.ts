//// [aDirectiveAboveAMultiLineMatchGovernsItsFirstLineOnly.tt] ////
variant S { A(n: number), B }
declare const s: S;
export function f() {
  // @ts-expect-error
  const v: string = match (s) {
    A(n) => n,
    B => "b",
  };
  // @ts-expect-error
  const w: number = match (s) { A(n) => n, B => "b" };
  return [v, w];
}


//// [aDirectiveAboveAMultiLineMatchGovernsItsFirstLineOnly.ts]
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
type S =
  | { kind: "A"; n: number }
  | { kind: "B" };
const S = {
  A: (n: number): S => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const s: S;
export function f() {
  // @ts-expect-error
  let $tt_v0: string; { const $tt_m = s; switch ($tt_m.kind) {
    case "A": { const { n } = $tt_m; $tt_v0 = n; break; }
    case "B": { $tt_v0 = "b"; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const v: string = $tt_v0;
  // @ts-expect-error
  let $tt_v1: number; { const $tt_m = s; switch ($tt_m.kind) { case "A": { const { n } = $tt_m; $tt_v1 = n; break; } case "B": { $tt_v1 = "b"; break; } default: { throw new Error("tt match: unexpected case " + $tt_show($tt_m)); } } } const w: number = $tt_v1;
  return [v, w];
}
