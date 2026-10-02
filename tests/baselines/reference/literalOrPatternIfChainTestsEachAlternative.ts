//// [literalOrPatternIfChainTestsEachAlternative.tt] ////
const v = match (s) { "a" | "b" if ok => 1, _ => 2 };


//// [literalOrPatternIfChainTestsEachAlternative.ts]
let $tt_v0$v: number;
{
  const $tt_m = s;
  do {
    if ($tt_m === "a" || $tt_m === "b") {
      if (ok) {
        $tt_v0$v = 1;
        break;
      }
    }
    $tt_v0$v = 2;
    break;
  } while (false);
}
const v = $tt_v0$v;
\ No newline at end of file
