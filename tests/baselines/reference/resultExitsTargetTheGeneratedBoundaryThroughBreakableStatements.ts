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
  $tt_v0$fromFor = { kind: "Ok" as const, value: $tt_t0.value };
  break $tt_v0$fromFor; } { $tt_v0$fromFor = { kind: "Ok" as const, value: 0 }; break $tt_v0$fromFor; }
}
const fromFor = $tt_v0$fromFor;
let $tt_v1$fromWhile;
$tt_v1$fromWhile: {
  while (ready()) { const $tt_t1 = read();
  if (!("value" in $tt_t1)) {
    $tt_v1$fromWhile = $tt_t1;
    break $tt_v1$fromWhile;
  }
  $tt_v1$fromWhile = { kind: "Ok" as const, value: $tt_t1.value };
  break $tt_v1$fromWhile; } { $tt_v1$fromWhile = { kind: "Ok" as const, value: 0 }; break $tt_v1$fromWhile; }
}
const fromWhile = $tt_v1$fromWhile;
let $tt_v2$fromDo;
$tt_v2$fromDo: {
  do { const $tt_t2 = read();
  if (!("value" in $tt_t2)) {
    $tt_v2$fromDo = $tt_t2;
    break $tt_v2$fromDo;
  }
  $tt_v2$fromDo = { kind: "Ok" as const, value: $tt_t2.value };
  break $tt_v2$fromDo; } while (ready()); { $tt_v2$fromDo = { kind: "Ok" as const, value: 0 }; break $tt_v2$fromDo; }
}
const fromDo = $tt_v2$fromDo;
let $tt_v3$fromSwitch;
$tt_v3$fromSwitch: {
  switch (tag) { default: const $tt_t3 = read();
  if (!("value" in $tt_t3)) {
    $tt_v3$fromSwitch = $tt_t3;
    break $tt_v3$fromSwitch;
  }
  $tt_v3$fromSwitch = { kind: "Ok" as const, value: $tt_t3.value };
  break $tt_v3$fromSwitch; } { $tt_v3$fromSwitch = { kind: "Ok" as const, value: 0 }; break $tt_v3$fromSwitch; }
}
const fromSwitch = $tt_v3$fromSwitch;
