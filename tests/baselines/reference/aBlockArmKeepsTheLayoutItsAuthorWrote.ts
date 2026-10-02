//// [aBlockArmKeepsTheLayoutItsAuthorWrote.tt] ////
variant E { A(n: number), B }
declare const e: E;
const v = match (e) {
  A(n) => {
    const m = n + 1;
    return m;
  },
  B => 0,
};


//// [aBlockArmKeepsTheLayoutItsAuthorWrote.ts]
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
  | { kind: "A"; n: number }
  | { kind: "B" };
const E = {
  A: (n: number): E => ({ kind: "A", n }),
  B: { kind: "B" } as const,
};
declare const e: E;
let $tt_v0$v: number;
{
  const $tt_m = e;
  switch ($tt_m.kind) {
    case "A": {
      const { n } = $tt_m;
      const m = n + 1;
      $tt_v0$v = m;
      break;
    }
    case "B": {
      $tt_v0$v = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const v = $tt_v0$v;
