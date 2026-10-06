//// [anOrPatternFieldMissingFromOneCaseIsReportedAtThatCase.tt] ////
variant V { A(n: number), B(m: string) }
export function f(v: V) {
  return match (v) { A(n) | B(n) => n };
}
export function g(v: V) {
  const A(n) | B(n) = v else { return 0; };
  return n;
}


//// [anOrPatternFieldMissingFromOneCaseIsReportedAtThatCase.ts]
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
type V =
  | { kind: "A"; n: number }
  | { kind: "B"; m: string };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: (m: string): V => ({ kind: "B", m }),
};
export function f(v: V) {
  let $tt_v0;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": case "B": {
        const { n } = $tt_m;
        $tt_v0 = n;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
}
export function g(v: V) {
  const $tt_t0 = v;
  if ($tt_t0.kind !== "A" && $tt_t0.kind !== "B") {
    return 0;
  }
  const { n } = $tt_t0;
  return n;
}
