//// [aSelectedArmValueAsADiscardedCommaOperandLeavesNothingBehind.tt] ////
variant S { A, B }
const log: string[] = [];
function o(): S { log.push("subject"); return S.A; }
function main(s: S, x: number) {
  const a = (match (s) { A => 1, B => 2 }, [x]);
  const b = (match (o()) { A => typeof 2, B => "b" }, x * 2);
  const c = (log.push("first"), match (o()) { A => 1, B => 2 }, x + 1);
  return [a, b, c];
}
console.log(JSON.stringify(main(S.A, 3)), log.join(","));


//// [aSelectedArmValueAsADiscardedCommaOperandLeavesNothingBehind.ts]
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
type S =
  | { kind: "A" }
  | { kind: "B" };
const S = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
const log: string[] = [];
function o(): S { log.push("subject"); return S.A; }
function main(s: S, x: number) {
  let $tt_v0: number;
  {
    const $tt_m = s;
    switch ($tt_m.kind) {
      case "A": {
        $tt_v0 = 1;
        break;
      }
      case "B": {
        $tt_v0 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const a = ( [x]);
  let $tt_v1: string;
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": {
        $tt_v1 = typeof 2;
        break;
      }
      case "B": {
        $tt_v1 = "b";
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const b = ( x * 2);
  let $tt_v2: number;
  (log.push("first"));
  {
    const $tt_m = o();
    switch ($tt_m.kind) {
      case "A": {
        $tt_v2 = 1;
        break;
      }
      case "B": {
        $tt_v2 = 2;
        break;
      }
      default: {
        throw new Error("tt match: unexpected case " + $tt_show($tt_m));
      }
    }
  }
  const c = (  x + 1);
  return [a, b, c];
}
console.log(JSON.stringify(main(S.A, 3)), log.join(","));
