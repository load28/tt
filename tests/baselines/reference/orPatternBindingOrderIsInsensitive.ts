//// [orPatternBindingOrderIsInsensitive.tt] ////
const r = match (p) { A(x, y) | B(y, x) => x + y, _ => 0 };


//// [orPatternBindingOrderIsInsensitive.ts]
let $tt_v0$r;
{
  const $tt_m = p;
  switch ($tt_m.kind) {
    case "A": case "B": {
      const { x, y } = $tt_m;
      $tt_v0$r = x + y;
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
