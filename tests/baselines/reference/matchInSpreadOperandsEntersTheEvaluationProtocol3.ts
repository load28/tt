//// [matchInSpreadOperandsEntersTheEvaluationProtocol3.tt] ////
consume(...match (kind) { A => [1], _ => [] });


//// [matchInSpreadOperandsEntersTheEvaluationProtocol3.ts]
{
  let $tt_v0: number;
  const $tt_v1 = (consume);
  {
    const $tt_m = kind;
    switch ($tt_m.kind) {
      case "A": $tt_v0 = 0; break;
      default: $tt_v0 = 1; break;
    }
  }
  $tt_v1(...($tt_v0 === 0 ? [1] : []));
}
