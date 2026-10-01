//// [nestedMatchSubjectUsesCollisionFreeSlots.tt] ////
variant V { A, B }
declare const v: V;
const n = match (match (v) { A => V.B, B => V.A }) { A => 1, B => 2 };


//// [nestedMatchSubjectUsesCollisionFreeSlots.ts]
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
  | { kind: "A" }
  | { kind: "B" };
const V = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
declare const v: V;
let $tt_v0$n: number;
{
  let $tt_m_1; {
    const $tt_m = v;
    switch ($tt_m.kind) {
      case "A": {
        $tt_m_1 = V.B;
        break;
      }
      case "B": {
        $tt_m_1 = V.A;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  switch ($tt_m_1.kind) {
    case "A": {
      $tt_v0$n = 1;
      break;
    }
    case "B": {
      $tt_v0$n = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m_1));
    }
  }
}
const n = $tt_v0$n;
