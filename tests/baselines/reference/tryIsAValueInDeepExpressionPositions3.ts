//// [tryIsAValueInDeepExpressionPositions3.tt] ////
function f(): TResult<string, string> { return Result.Ok(`v=${try read()}`); }


//// [tryIsAValueInDeepExpressionPositions3.ts]
function f(): TResult<string, string> { let $tt_v0;
const $tt_t0 = read();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
return Result.Ok(`v=${$tt_v0}`); }
