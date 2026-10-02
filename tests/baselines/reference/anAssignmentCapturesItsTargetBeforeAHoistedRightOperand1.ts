//// [anAssignmentCapturesItsTargetBeforeAHoistedRightOperand1.tt] ////
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function read(): R;
declare function target(): { v: number };
declare function key(): "v";
declare let state: { v: number };
export function f(): R {
  target()[key()] -= try read();
  return { kind: "Ok", value: 0 };
}


//// [anAssignmentCapturesItsTargetBeforeAHoistedRightOperand1.ts]
type R = { kind: "Ok"; value: number } | { kind: "Err"; error: string };
declare function read(): R;
declare function target(): { v: number };
declare function key(): "v";
declare let state: { v: number };
export function f(): R {
  let $tt_v0: number;
  const $tt_v1 = (target());
  const $tt_v2 = (key());
  let $tt_v3 = ($tt_v1[$tt_v2]);
  const $tt_t0 = read();
  if (!("value" in $tt_t0)) {
    return $tt_t0;
  }
  $tt_v0 = $tt_t0.value;
  $tt_v1[$tt_v2] = $tt_v3 -= $tt_v0;
  return { kind: "Ok", value: 0 };
}
