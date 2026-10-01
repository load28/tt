//// [tryInSpreadOperandsEntersTheEvaluationProtocol1.tt] ////
function f() { const value = { ...try read() }; }


//// [tryInSpreadOperandsEntersTheEvaluationProtocol1.ts]
function f() { let $tt_v0;
const $tt_t0 = read();
if (!("value" in $tt_t0)) {
  return $tt_t0;
}
$tt_v0 = $tt_t0.value;
const value = { ...$tt_v0 }; }
