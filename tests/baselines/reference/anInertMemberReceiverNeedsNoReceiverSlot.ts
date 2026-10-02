//// [anInertMemberReceiverNeedsNoReceiverSlot.tt] ////
variant E { A(value: string), B }
const value = "abc".replace(match (E.A("a")) { A(value) => value, B => "b" },"x",);


//// [anInertMemberReceiverNeedsNoReceiverSlot.ts]
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
  | { kind: "A"; value: string }
  | { kind: "B" };
const E = {
  A: (value: string): E => ({ kind: "A", value }),
  B: { kind: "B" } as const,
};
let $tt_v0$value: string | RegExp;
{
  const $tt_m = E.A("a");
  switch ($tt_m.kind) {
    case "A": {
      const { value } = $tt_m;
      $tt_v0$value = value;
      break;
    }
    case "B": {
      $tt_v0$value = "b";
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const value = "abc".replace($tt_v0$value,"x",);
