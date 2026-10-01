//// [literalMatchWithoutAWildcardGetsARuntimeGuard.tt] ////
const label = match (dir) { "a" => 1, "b" => 2 };


//// [literalMatchWithoutAWildcardGetsARuntimeGuard.ts]
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
let $tt_v0$label: number;
{
  const $tt_m = dir;
  switch ($tt_m) {
    case "a": {
      $tt_v0$label = 1;
      break;
    }
    case "b": {
      $tt_v0$label = 2;
      break;
    }
    default: {
      throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
    }
  }
}
const label = $tt_v0$label;
\ No newline at end of file
