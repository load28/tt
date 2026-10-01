//// [anAssignmentCapturesItsTargetBeforeAHoistedRightOperand2.tt] ////
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function read(): R;
declare function target(): { v: number };
declare function key(): "v";
declare let state: { v: number };
export function g(): R {
  state.v *= try read();
  this.v = try read();
  return { kind: "Ok", value: 0 };
}


//// [anAssignmentCapturesItsTargetBeforeAHoistedRightOperand2.ts]
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function read(): R;
declare function target(): { v: number };
declare function key(): "v";
declare let state: { v: number };
export function g(): R {
  let $tt_v0: number;
  let $tt_v1 = (state.v);
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  state.v = $tt_v1 *= $tt_v0;
  let $tt_v2: number;
  const $tt_t1 = read();
  if (!("value" in $tt_t1)) {
    return $tt_t1;
  }
  $tt_v2 = $tt_t1.value;
  this.v = $tt_v2;
  return { kind: "Ok", value: 0 };
}
