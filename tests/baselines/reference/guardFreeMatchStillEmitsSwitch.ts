//// [guardFreeMatchStillEmitsSwitch.tt] ////
const r = match (x) { A => 1, _ => 0 };


//// [guardFreeMatchStillEmitsSwitch.ts]
let $tt_v0$r: number;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      $tt_v0$r = 1;
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
