//// [aScriptValueKeepsAVarItDeclaresInTheGlobalScope.tt] ////
declare const o: { kind: "A" } | { kind: "B" };
const total = match (o) { A => { var seen = 1; return seen; }, B => 2 };


//// [aScriptValueKeepsAVarItDeclaresInTheGlobalScope.ts]
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
declare const o: { kind: "A" } | { kind: "B" };
let $tt_v0$total: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      var seen = 1; $tt_v0$total = seen; break;
    }
    case "B": {
      $tt_v0$total = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const total = $tt_v0$total;
