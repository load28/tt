//// [guardWithOrPatternEmitsCombinedCondition.tt] ////
const r = match (x) { A(v) | B(v) if v > 0 => v, _ => 0 };


//// [guardWithOrPatternEmitsCombinedCondition.ts]
let $tt_v0$r;
{
  const $tt_m = x;
  do {
    if ($tt_m.kind === "A" || $tt_m.kind === "B") {
      const { v } = $tt_m;
      if (v > 0) {
        $tt_v0$r = v;
        break;
      }
    }
    $tt_v0$r = 0;
    break;
  } while (false);
}
const r = $tt_v0$r;
\ No newline at end of file
