//// [literalMatchKeepsTheNumberSpellingOfTheSource.tt] ////
const v = match (x) { 0xff => 1, 1_000 => 2, 1.5e2 => 3, -1 => 4, _ => 0 };


//// [literalMatchKeepsTheNumberSpellingOfTheSource.ts]
let $tt_v0$v: number;
{
  const $tt_m = x;
  switch ($tt_m) {
    case 0xff: {
      $tt_v0$v = 1;
      break;
    }
    case 1_000: {
      $tt_v0$v = 2;
      break;
    }
    case 1.5e2: {
      $tt_v0$v = 3;
      break;
    }
    case -1: {
      $tt_v0$v = 4;
      break;
    }
    default: {
      $tt_v0$v = 0;
      break;
    }
  }
}
const v = $tt_v0$v;
\ No newline at end of file
