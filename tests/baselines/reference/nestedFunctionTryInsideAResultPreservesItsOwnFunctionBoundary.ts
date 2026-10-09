//// [nestedFunctionTryInsideAResultPreservesItsOwnFunctionBoundary.tt] ////
const value = result { const inner = () => { return try step(); }; return try inner(); };


//// [nestedFunctionTryInsideAResultPreservesItsOwnFunctionBoundary.ts]
let $tt_v0$value;
$tt_v0$value: {
  const inner = () => { let $tt_v1;
  const $tt_t0 = step();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v1 = $tt_t0.value;
  return $tt_v1; }; const $tt_t1 = inner();
  if (!("value" in $tt_t1)) {
    $tt_v0$value = $tt_t1;
    break $tt_v0$value;
  }
  $tt_v0$value = { kind: "Ok" as const, value: $tt_t1.value };
  break $tt_v0$value;
}
const value = $tt_v0$value;
