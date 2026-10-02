//// [aCandidateTheArmsSatisfyMakesTheMatchExhaustive1.tt] ////
variant Big { A(s: string), B, C }
variant Small { A(s: string), B }
const f = (v: Small) => match (v) { A(s) => s, B => "b" };


//// [aCandidateTheArmsSatisfyMakesTheMatchExhaustive1.ts]
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
type Big =
  | { kind: "A"; s: string }
  | { kind: "B" }
  | { kind: "C" };
const Big = {
  A: (s: string): Big => ({ kind: "A", s }),
  B: { kind: "B" } as const,
  C: { kind: "C" } as const,
};
type Small =
  | { kind: "A"; s: string }
  | { kind: "B" };
const Small = {
  A: (s: string): Small => ({ kind: "A", s }),
  B: { kind: "B" } as const,
};
const f = (v: Small) => {
  let $tt_v0: string;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { s } = $tt_m;
        $tt_v0 = s;
        break;
      }
      case "B": {
        $tt_v0 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return $tt_v0;
};
