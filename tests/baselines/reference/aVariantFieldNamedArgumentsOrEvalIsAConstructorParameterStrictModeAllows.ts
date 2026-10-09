//// [main.tt] ////
variant V { A(arguments: number, public: boolean, eval?: string), B }
const a = V.A(1, true);
const b = V.A(2, false, "x");
function show(v: V): string {
  return match (v) {
    A(arguments: n, public: p, eval: e) => `${n} ${p} ${e ?? "-"}`,
    B => "b",
  };
}
console.log(show(a), show(b), "eval" in a, JSON.stringify(b));


//// [main.ts]
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
type V =
  | { kind: "A"; arguments: number; public: boolean; eval?: string }
  | { kind: "B" };
const V = {
  A: ($tt_arguments: number, $tt_public: boolean, $tt_eval?: string): V => ({ kind: "A", arguments: $tt_arguments, public: $tt_public, ...($tt_eval === undefined ? {} : { eval: $tt_eval }) }),
  B: { kind: "B" } as const,
};
const a = V.A(1, true);
const b = V.A(2, false, "x");
function show(v: V): string {
  let $tt_v0: string;
  {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        const { arguments: n, public: p, eval: e } = $tt_m;
        $tt_v0 = `${n} ${p} ${e ?? "-"}`;
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
}
console.log(show(a), show(b), "eval" in a, JSON.stringify(b));
