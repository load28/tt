//// [resultKeepsControlTransfersOwnedByItsOwnLoop.tt] ////
const value = result { const item = try read(); while (item) { break; } return item; };


//// [resultKeepsControlTransfersOwnedByItsOwnLoop.ts]
let $tt_v0$value;
$tt_v0$value: {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    $tt_v0$value = $tt_t0;
    break $tt_v0$value;
  }
  const item = $tt_t0.value; while (item) { break; } { const $tt_a0 = { value: { kind: "Ok" as const, value: item } }; $tt_v0$value = $tt_a0.value; break $tt_v0$value; }
}
const value = $tt_v0$value;
