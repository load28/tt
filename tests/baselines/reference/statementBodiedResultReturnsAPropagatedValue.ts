//// [statementBodiedResultReturnsAPropagatedValue.tt] ////
const value = result { return try read(); };


//// [statementBodiedResultReturnsAPropagatedValue.ts]
let $tt_v0$value;
$tt_v0$value: {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    $tt_v0$value = $tt_t0;
    break $tt_v0$value;
  }
  $tt_v0$value = { kind: "Ok" as const, value: $tt_t0.value };
  break $tt_v0$value;
}
const value = $tt_v0$value;
