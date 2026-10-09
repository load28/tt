//// [aNestedConciseArrowBodyClosesBeforeItsOuterArrow.tt] ////
variant V { A, B }
const outer = (v: V) => [match (v) { A => () => [0], B => () => [2] }, () => [match (v) { A => 3, B => 4 }]];
const [first, inner] = outer(V.B);
console.log(first(), inner());
export {};


//// [aNestedConciseArrowBodyClosesBeforeItsOuterArrow.ts]
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
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const outer = (v: V) => {
  let $tt_v0: number;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      case "B": $tt_v0 = 1; break;
      default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
  return [($tt_v0 === 0 ? () => [0] : () => [2]), () => {
    let $tt_v1: number;
    {
      const $tt_m = v;
      switch ($tt_m.kind) {
        case "A": $tt_v1 = 0; break;
        case "B": $tt_v1 = 1; break;
        default: throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
    return [($tt_v1 === 0 ? 3 : 4)];
  }];
};
const [first, inner] = outer(V.B);
console.log(first(), inner());
export {};
