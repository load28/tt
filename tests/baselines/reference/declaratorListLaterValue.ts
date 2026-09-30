//// [declaratorListLaterValue.tt] ////
// Repro from TASK-593
variant O { A(n: number), B }
declare const o: O;
declare function t(s: string): number;
export function f() {
  const a = t("a"), b = match (o) { A(n) => a + n, B => 0 };
  return b;
}
export function g() {
  var a = 10, b = match (o) { A(n) => a + n, B => 0 };
  return b;
}


//// [declaratorListLaterValue.ts]
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
// Repro from TASK-593
type O =
  | { kind: "A"; n: number }
  | { kind: "B" };
const O = {
  A: (n: number): O => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const o: O;
declare function t(s: string): number;
export function f() {
  const a = t("a");
  let $tt_v0: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = a + n;
        break;
      }
      case "B": {
        $tt_v0 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const b = $tt_v0;
  return b;
}
export function g() {
  var a = 10;
  let $tt_v1: number;
  {
    const $tt_m = o;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = a + n;
        break;
      }
      case "B": {
        $tt_v1 = 0;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  var b = $tt_v1;
  return b;
}
