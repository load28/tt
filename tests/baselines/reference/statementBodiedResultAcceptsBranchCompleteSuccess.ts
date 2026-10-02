//// [statementBodiedResultAcceptsBranchCompleteSuccess.tt] ////
const value = result { const item = try read(); if (item) return item; else return 0; };


//// [statementBodiedResultAcceptsBranchCompleteSuccess.ts]
let $tt_v0$value;
$tt_v0$value: {
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    $tt_v0$value = $tt_t0;
    break $tt_v0$value;
  }
  const item = $tt_t0.value; if (item) { const $tt_a0 = { value: { kind: "Ok" as const, value: item } }; $tt_v0$value = $tt_a0.value; break $tt_v0$value; } else { const $tt_a1 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v0$value = $tt_a1.value; break $tt_v0$value; }
}
const value = $tt_v0$value;
