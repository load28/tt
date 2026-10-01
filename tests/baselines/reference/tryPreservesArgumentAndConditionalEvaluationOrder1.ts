//// [tryPreservesArgumentAndConditionalEvaluationOrder1.tt] ////
function f(flag: boolean): TResult<number, string> {
  return Result.Ok(call(first(), flag && try second(), third()));
}


//// [tryPreservesArgumentAndConditionalEvaluationOrder1.ts]
function f(flag: boolean): TResult<number, string> {
  let $tt_v5;
  const $tt_v2 = (call);
  const $tt_v3 = (first());
  let $tt_v1: boolean;
  if ($tt_v1 = flag) {
    let $tt_v0;
    const $tt_t0 = second();
    if (!("value" in $tt_t0)) {
      return $tt_t0;
    }
    $tt_v0 = $tt_t0.value;
    $tt_v5 = $tt_v1 && $tt_v0;
  } else {
    $tt_v5 = $tt_v1;
  }
  
  return Result.Ok($tt_v2($tt_v3, $tt_v5, third()));
}
