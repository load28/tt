//// [anIfLetThatStartsAStatementStaysAStatement6.tt] ////
variant O { Some(value: number), None }
declare const o: O;
declare function g(x: unknown): number;
const x = match (o) { Some(value) => { if let Some(value: v) = o { g(v); } return 1; }, None => 0 };


//// [anIfLetThatStartsAStatementStaysAStatement6.ts]
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
type O =
  | { kind: "Some"; value: number }
  | { kind: "None" };
const O = {
  Some: (value: number): O => ({ kind: "Some", value }),
  None: { kind: "None" } as const,
};
declare const o: O;
declare function g(x: unknown): number;
let $tt_v0$x: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "Some": {
      const { value } = $tt_m;
      {
        const $tt_t0 = o;
        if ($tt_t0.kind === "Some") {
          const { value: v } = $tt_t0;
          g(v);
        }
      } $tt_v0$x = 1; break;
    }
    case "None": {
      $tt_v0$x = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const x = $tt_v0$x;
