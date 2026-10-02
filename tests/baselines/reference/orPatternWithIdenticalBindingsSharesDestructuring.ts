//// [orPatternWithIdenticalBindingsSharesDestructuring.tt] ////
const r = match (x) { A(v) | B(v) => v, _ => 0 };


//// [orPatternWithIdenticalBindingsSharesDestructuring.ts]
let $tt_v0$r;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": case "B": {
      const { v } = $tt_m;
      $tt_v0$r = v;
      break;
    }
    default: {
      $tt_v0$r = 0;
      break;
    }
  }
}
const r = $tt_v0$r;
\ No newline at end of file
