//// [resultExitsTargetTheGeneratedBoundaryThroughBreakableStatements.tt] ////

const fromFor = result { for (const item of items) { return try read(item); } return 0; };
const fromWhile = result { while (ready()) { return try read(); } return 0; };
const fromDo = result { do { return try read(); } while (ready()); return 0; };
const fromSwitch = result { switch (tag) { default: return try read(); } return 0; };


//// [resultExitsTargetTheGeneratedBoundaryThroughBreakableStatements.ts]

let $tt_v0$fromFor;
$tt_v0$fromFor: {
  for (const item of items) { const $tt_t0 = read(item);
  if (!("value" in $tt_t0)) {
    $tt_v0$fromFor = $tt_t0;
    break $tt_v0$fromFor;
  }
  const $tt_a0 = { value: { kind: "Ok" as const, value: $tt_t0.value } };
  $tt_v0$fromFor = $tt_a0.value;
  break $tt_v0$fromFor; } { const $tt_a1 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v0$fromFor = $tt_a1.value; break $tt_v0$fromFor; }
}
const fromFor = $tt_v0$fromFor;
let $tt_v1$fromWhile;
$tt_v1$fromWhile: {
  while (ready()) { const $tt_t1 = read();
  if (!("value" in $tt_t1)) {
    $tt_v1$fromWhile = $tt_t1;
    break $tt_v1$fromWhile;
  }
  const $tt_a2 = { value: { kind: "Ok" as const, value: $tt_t1.value } };
  $tt_v1$fromWhile = $tt_a2.value;
  break $tt_v1$fromWhile; } { const $tt_a3 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v1$fromWhile = $tt_a3.value; break $tt_v1$fromWhile; }
}
const fromWhile = $tt_v1$fromWhile;
let $tt_v2$fromDo;
$tt_v2$fromDo: {
  do { const $tt_t2 = read();
  if (!("value" in $tt_t2)) {
    $tt_v2$fromDo = $tt_t2;
    break $tt_v2$fromDo;
  }
  const $tt_a4 = { value: { kind: "Ok" as const, value: $tt_t2.value } };
  $tt_v2$fromDo = $tt_a4.value;
  break $tt_v2$fromDo; } while (ready()); { const $tt_a5 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v2$fromDo = $tt_a5.value; break $tt_v2$fromDo; }
}
const fromDo = $tt_v2$fromDo;
let $tt_v3$fromSwitch;
$tt_v3$fromSwitch: {
  switch (tag) { default: const $tt_t3 = read();
  if (!("value" in $tt_t3)) {
    $tt_v3$fromSwitch = $tt_t3;
    break $tt_v3$fromSwitch;
  }
  const $tt_a6 = { value: { kind: "Ok" as const, value: $tt_t3.value } };
  $tt_v3$fromSwitch = $tt_a6.value;
  break $tt_v3$fromSwitch; } { const $tt_a7 = { value: { kind: "Ok" as const, value: 0 } }; $tt_v3$fromSwitch = $tt_a7.value; break $tt_v3$fromSwitch; }
}
const fromSwitch = $tt_v3$fromSwitch;
