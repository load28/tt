//// [matchInsideTemplateInterpolation.tt] ////
const s = `v=${match (x) { A => 1, _ => 0 }}`;


//// [matchInsideTemplateInterpolation.ts]
let $tt_v0$s: number;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": $tt_v0$s = 0; break;
    default: $tt_v0$s = 1; break;
  }
}
const s = `v=${($tt_v0$s === 0 ? 1 : 0)}`;
\ No newline at end of file
