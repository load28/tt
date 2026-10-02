//// [aCapturedOperandCarriesAnEarlierSiblingValueByItsSlot.tt] ////
declare function a(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function h() { return (try a()).toFixed(1).length + (try a()); }


//// [aCapturedOperandCarriesAnEarlierSiblingValueByItsSlot.ts]
declare function a(): { kind: "Ok"; value: number } | { kind: "Err"; error: string };
function h() { let $tt_v0: number;
let $tt_v1: number;
const $tt_t0 = a();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
const $tt_v2 = (($tt_v0).toFixed(1).length);
const $tt_t1 = a();
if (!("value" in $tt_t1)) {
  return $tt_t1;
}
$tt_v1 = $tt_t1.value;
return $tt_v2 + ($tt_v1); }
