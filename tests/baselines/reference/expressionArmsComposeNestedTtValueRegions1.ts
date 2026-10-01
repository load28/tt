//// [expressionArmsComposeNestedTtValueRegions1.tt] ////
variant V { A(n: number), B }
declare const v: V; declare const w: V;
const text = match (v) { A(n) => `${match (w) { A(m) => m, B => n }}`, B => "" };


//// [expressionArmsComposeNestedTtValueRegions1.ts]
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
declare const v: V; declare const w: V;
let $tt_v0$text: string;
{
  const $tt_m = v;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      let $tt_v1$text;
      {
        const $tt_m = w;
        switch ($tt_m.kind) {
          case "A": {
            const { m } = $tt_m;
            $tt_v1$text = m;
            break;
          }
          case "B": {
            $tt_v1$text = n;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      $tt_v0$text = `${$tt_v1$text}`;
      break;
    }
    case "B": {
      $tt_v0$text = "";
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const text = $tt_v0$text;
