//// [aSharedTagNameDoesNotDragAnUnrelatedUnionIntoAVariant.tt] ////
variant Shape { Circle(radius: number), Empty }
type Msg = { kind: "Empty" } | { kind: "Full"; n: number };
const a = match (m) { Empty => 0, Full(n) => n };


//// [aSharedTagNameDoesNotDragAnUnrelatedUnionIntoAVariant.ts]
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
type Shape =
  | { kind: "Circle"; radius: number }
  | { kind: "Empty" };
const Shape = {
  Circle: (radius: number): Shape => ({ kind: "Circle", radius }),
  Empty: { kind: "Empty" } as const,
};
type Msg = { kind: "Empty" } | { kind: "Full"; n: number };
let $tt_v0$a;
{
  const $tt_m = m;
  switch ($tt_m.kind) {
    case "Empty": {
      $tt_v0$a = 0;
      break;
    }
    case "Full": {
      const { n } = $tt_m;
      $tt_v0$a = n;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const a = $tt_v0$a;
