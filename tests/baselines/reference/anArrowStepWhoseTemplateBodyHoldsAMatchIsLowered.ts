//// [anArrowStepWhoseTemplateBodyHoldsAMatchIsLowered.tt] ////
variant V { A(n: number), B }
const label = (v: V) => v |> (w => `value: ${match (w) { A(n) => n, B => "none" }}`);
console.log(label(V.A(3)), label(V.B));


//// [anArrowStepWhoseTemplateBodyHoldsAMatchIsLowered.ts]
var $tt_ap: <A, B>(v: A, f: (v: A) => B) => B = function (v, f) {
  return f(v);
};
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
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
const label = (v: V) => $tt_ap(v, ((w => {
  let $tt_v0: (number) | (string);
  {
    const $tt_m = w;
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v0 = n;
        break;
      }
      case "B": {
        $tt_v0 = "none";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  return `value: ${$tt_v0}`;
})));
console.log(label(V.A(3)), label(V.B));
