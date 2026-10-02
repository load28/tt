//// [matchInSpreadOperandsEntersTheEvaluationProtocol2.tt] ////
const value = [ ...match (kind) { A => [1], _ => [] } ];


//// [matchInSpreadOperandsEntersTheEvaluationProtocol2.ts]
let $tt_v0$value: number;
{
  const $tt_m = kind;
  switch ($tt_m.kind) {
    case "A": $tt_v0$value = 0; break;
    default: $tt_v0$value = 1; break;
  }
}
const value = [ ...($tt_v0$value === 0 ? [1] : []) ];
