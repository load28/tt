//// [literalMatchWithoutAWildcardHasNoIfChainCaseGuard.tt] ////
const v = match (code) { 200 if ok => 1, 404 => 2 };


//// [literalMatchWithoutAWildcardHasNoIfChainCaseGuard.ts]
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
let $tt_v0$v: number;
{
  const $tt_m = code;
  do {
    if ($tt_m === 200) {
      if (ok) {
        $tt_v0$v = 1;
        break;
      }
    }
    if ($tt_m === 404) {
      $tt_v0$v = 2;
      break;
    }
    throw new Error("tt match: unexpected literal " + $tt_show($tt_m));
  } while (false);
}
const v = $tt_v0$v;
\ No newline at end of file
