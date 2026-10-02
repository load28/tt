//// [tryIsAValueInDeepExpressionPositions5.tt] ////
function f(r: R): TResult<number, string> { return match (r) { A => try total(), B => Result.Ok(0) }; }


//// [tryIsAValueInDeepExpressionPositions5.ts]
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
function f(r: R): TResult<number, string> { let $tt_v0: TResult<number, string>;
{
  const $tt_m = r;
  switch ($tt_m.kind) {
    case "A": {
      const $tt_t0 = total();
      if (!("value" in $tt_t0)) {
        return $tt_t0;
      }
      $tt_v0 = $tt_t0.value;
      break;
    }
    case "B": {
      $tt_v0 = Result.Ok(0);
      break;
    }
    default: {
      throw new Error("tt match: unexpected case " + $tt_show($tt_m));
    }
  }
}
return $tt_v0; }
