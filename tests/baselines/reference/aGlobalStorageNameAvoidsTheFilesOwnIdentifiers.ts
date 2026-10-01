//// [aGlobalStorageNameAvoidsTheFilesOwnIdentifiers.tt] ////
declare const $tt_v0$total: number;
declare const o: { kind: "A" } | { kind: "B" };
const total = match (o) { A => $tt_v0$total, B => 2 };


//// [aGlobalStorageNameAvoidsTheFilesOwnIdentifiers.ts]
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
declare const $tt_v0$total: number;
declare const o: { kind: "A" } | { kind: "B" };
let $tt_v0_1$total: number;
{
  const $tt_m = o;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v0_1$total = $tt_v0$total;
      break;
    }
    case "B": {
      $tt_v0_1$total = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
const total = $tt_v0_1$total;
