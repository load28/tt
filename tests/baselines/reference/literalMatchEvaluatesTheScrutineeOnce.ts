//// [literalMatchEvaluatesTheScrutineeOnce.tt] ////
const v = match (getValue()) { "a" => foo(), _ => bar() };


//// [literalMatchEvaluatesTheScrutineeOnce.ts]
let $tt_v0$v;
{
  const $tt_m = getValue();
  switch ($tt_m) {
    case "a": {
      $tt_v0$v = foo();
      break;
    }
    default: {
      $tt_v0$v = bar();
      break;
    }
  }
}
const v = $tt_v0$v;
\ No newline at end of file
