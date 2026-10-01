//// [tryIsAValueInDeepExpressionPositions1.tt] ////
function f(): TResult<number, string> {
  return Result.Ok(Math.round(try total() * 1.1));
}


//// [tryIsAValueInDeepExpressionPositions1.ts]
function f(): TResult<number, string> {
  let $tt_v0;
  const $tt_t0 = total();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  return Result.Ok(Math.round($tt_v0 * 1.1));
}
