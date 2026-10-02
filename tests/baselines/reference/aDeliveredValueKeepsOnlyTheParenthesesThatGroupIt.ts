//// [aDeliveredValueKeepsOnlyTheParenthesesThatGroupIt.tt] ////
variant E { A(v: number), B }
declare const e: E;
const plain = match (e) { A(v) => v + 1, B => 0 };
const seq = match (e) { A(v) => (v, v + 1), B => 0 };


//// [aDeliveredValueKeepsOnlyTheParenthesesThatGroupIt.ts]
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
type E =
  | { kind: "A"; v: number }
  | { kind: "B" };
const E = {
  A: (v: number): E => ({ kind: "A", v }),
  B: { kind: "B" } as const,
};
declare const e: E;
let $tt_v0$plain: number;
{
  const $tt_m = e;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v0$plain = v + 1;
      break;
    }
    case "B": {
      $tt_v0$plain = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const plain = $tt_v0$plain;
let $tt_v1$seq: number;
{
  const $tt_m = e;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v1$seq = (v, v + 1);
      break;
    }
    case "B": {
      $tt_v1$seq = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const seq = $tt_v1$seq;
