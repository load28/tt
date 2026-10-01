//// [tryIsAValueInDeepExpressionPositions2.tt] ////
function f(flag: boolean): TResult<number, string> { const value = try (flag ? left() : right()); return Result.Ok(value); }


//// [tryIsAValueInDeepExpressionPositions2.ts]
function f(flag: boolean): TResult<number, string> { const $tt_t0 = (flag ? left() : right());
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
const value = $tt_t0.value; return Result.Ok(value); }
