//// [commaExpressionScrutineeIsStillASingleMatch.tt] ////
const r = match ((a, b)) { A => 1, _ => 0 };
const s = match (a, b) { A => 1, _ => 0 };


//// [commaExpressionScrutineeIsStillASingleMatch.ts]
let $tt_v0$r: number;
{
  const $tt_m = (a, b);
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
let $tt_v1$s: number;
{
  const $tt_m = (a, b);
  switch ($tt_m.kind) {
    case "A": {
      $tt_v1$s = 1;
      break;
    }
    default: {
      $tt_v1$s = 0;
      break;
    }
  }
}
const s = $tt_v1$s;
