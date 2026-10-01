//// [resultReturnExpressionPropagatesToTheResultScope.tt] ////
const value = result { return Math.round(try total() * 1.1); };


//// [resultReturnExpressionPropagatesToTheResultScope.ts]
let $tt_v0$value;
$tt_v0$value: {
  let $tt_v1;
  const $tt_t0 = total();
  if (!("value" in $tt_t0)) {
    $tt_v0$value = $tt_t0;
    break $tt_v0$value;
  }
  $tt_v1 = $tt_t0.value;
  { const $tt_a0 = { value: { kind: "Ok" as const, value: Math.round($tt_v1 * 1.1) } }; $tt_v0$value = $tt_a0.value; break $tt_v0$value; }
}
const value = $tt_v0$value;
