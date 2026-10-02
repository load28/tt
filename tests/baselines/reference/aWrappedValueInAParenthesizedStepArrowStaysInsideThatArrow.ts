//// [aWrappedValueInAParenthesizedStepArrowStaysInsideThatArrow.tt] ////
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
variant V { A(n: number), B }
declare function next(): R<number>;
declare const value: number;
const f = value |> (x => (try next()));
const h = value |> ((v: number) => (match (V.A(v)) { A(n) => n, B => 0 }) as number);


//// [aWrappedValueInAParenthesizedStepArrowStaysInsideThatArrow.ts]
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
type R<T> = { kind: "Ok"; value: T } | { kind: "Err"; error: string };
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare function next(): R<number>;
declare const value: number;
const f = $tt_ap(value, ((x => {
  let $tt_v0: number | {
    kind: "Err";
    error: string;
};
  const $tt_t0 = next();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  return ($tt_v0);
})));
const h = $tt_ap(value, (((v: number) => {
  let $tt_v1: number;
  {
    const $tt_m = V.A(v);
    switch ($tt_m.kind) {
      case "A": {
        const { n } = $tt_m;
        $tt_v1 = n;
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
  return ($tt_v1) as number;
})));
