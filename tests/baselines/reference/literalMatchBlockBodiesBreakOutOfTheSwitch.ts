//// [literalMatchBlockBodiesBreakOutOfTheSwitch.tt] ////
const v = match (s) { "a" => { return 1; }, _ => 0 };


//// [literalMatchBlockBodiesBreakOutOfTheSwitch.ts]
let $tt_v0$v: number;
{
  const $tt_m = s;
  switch ($tt_m) {
    case "a": {
      $tt_v0$v = 1; break;
    }
    default: {
      $tt_v0$v = 0;
      break;
    }
  }
}
const v = $tt_v0$v;
\ No newline at end of file
