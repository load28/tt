//// [nestedInitializerMatchInheritsTheParentAssignmentContinuation.tt] ////
variant Outer { A, B }
enum Inner { X, Y }
const value = match (outer) { A => match (inner) { X => 1, Y => 2 }, B => 0 };


//// [nestedInitializerMatchInheritsTheParentAssignmentContinuation.ts]
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
type Outer =
  | { kind: "A" }
  | { kind: "B" };
const Outer = {
  A: { kind: "A" } as const,
  B: { kind: "B" } as const,
};
enum Inner { X, Y }
let $tt_v0$value: number;
{
  const $tt_m = outer;
  switch ($tt_m.kind) {
    case "A": {
      {
        const $tt_m = inner;
        switch ($tt_m.kind) {
          case "X": {
            $tt_v0$value = 1;
            break;
          }
          case "Y": {
            $tt_v0$value = 2;
            break;
          }
          default: {
            throw new Error("tt match: unexpected case " + $tt_show($tt_m));
          }
        }
      }
      break;
    }
    case "B": {
      $tt_v0$value = 0;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const value = $tt_v0$value;
