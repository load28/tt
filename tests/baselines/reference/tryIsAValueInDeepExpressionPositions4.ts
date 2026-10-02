//// [tryIsAValueInDeepExpressionPositions4.tt] ////
function f(): TResult<{ amount: number }, string> { return Result.Ok({ amount: try total() }); }


//// [tryIsAValueInDeepExpressionPositions4.ts]
function f(): TResult<{ amount: number }, string> { let $tt_v0;
const $tt_t0 = total();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
return Result.Ok({ amount: $tt_v0 }); }
