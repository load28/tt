//// [tryPreservesArgumentAndConditionalEvaluationOrder2.tt] ////
function f(maybe: any): TResult<number, string> { return Result.Ok(maybe?.(first(), try second(), third())); }


//// [tryPreservesArgumentAndConditionalEvaluationOrder2.ts]
function f(maybe: any): TResult<number, string> { let $tt_v4;
const $tt_v1: typeof maybe = (maybe);
if ($tt_v1 != null) {
  const $tt_v2 = (first());
  let $tt_v0;
  const $tt_t0 = second();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  $tt_v4 = $tt_v1($tt_v2, $tt_v0, third());
} else {
  $tt_v4 = undefined;
}

return Result.Ok($tt_v4); }
