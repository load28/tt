//// [resultExitLabelAvoidsUserIdentifiersAndLabels.tt] ////
function run() { $tt_v0: while (ready()) { break $tt_v0; } const $tt_v1 = 0; const value = result { const item = try read(); return item; }; return value; }


//// [resultExitLabelAvoidsUserIdentifiersAndLabels.ts]
function run() { $tt_v0: while (ready()) { break $tt_v0; } const $tt_v1 = 0; let $tt_v0_1;
$tt_v0_1: {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    $tt_v0_1 = $tt_t0;
    break $tt_v0_1;
  }
  const item = $tt_t0.value; { const $tt_a0 = { value: { kind: "Ok" as const, value: item } }; $tt_v0_1 = $tt_a0.value; break $tt_v0_1; }
}
const value = $tt_v0_1; return value; }
