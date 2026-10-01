//// [expressionArmsComposeNestedTtValueRegions2.tt] ////
variant V { A(n: number), B } variant R { Ok(value: number), Err(error: string) }
declare const v: V; declare const g: () => R; declare const unwrap: (r: R) => number;
const n = match (v) { A(n) => unwrap(result { const x = try g(); return n + x; }), B => -1 };


//// [expressionArmsComposeNestedTtValueRegions2.ts]
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
var $tt_expr: <T>(run: () => T) => T = function (run) { return run(); };
type V =
  | { kind: "A"; n: number }
  | { kind: "B" };
const V = {
  A: (n: number): V => ({ kind: "A", n }),
  B: { kind: "B" } as const,
}; type R =
  | { kind: "Ok"; value: number }
  | { kind: "Err"; error: string };
const R = {
  Ok: (value: number): R => ({ kind: "Ok", value }),
  Err: (error: string): R => ({ kind: "Err", error }),
};
declare const v: V; declare const g: () => R; declare const unwrap: (r: R) => number;
let $tt_v0$n: number;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      $tt_v0$n = (unwrap($tt_expr(() => {
        const $tt_t0 = g();
        if (!("value" in $tt_t0)) {
          return $tt_t0;
        }
        const x = $tt_t0.value; { return { kind: "Ok" as const, value: n + x }; }
        })));
      break;
    }
    case "B": {
      $tt_v0$n = -1;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const n = $tt_v0$n;
