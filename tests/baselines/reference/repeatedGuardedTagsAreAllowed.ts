//// [repeatedGuardedTagsAreAllowed.tt] ////
const r = match (x) { A(v) if v > 9 => 2, A(v) if v > 0 => 1, A => 0, _ => -1 };


//// [repeatedGuardedTagsAreAllowed.ts]
let $tt_v0$r: number;
{
  const $tt_m = x;
  do {
    if ($tt_m.kind === "A") {
      const { v } = $tt_m;
      if (v > 9) {
        $tt_v0$r = 2;
        break;
      }
    }
    if ($tt_m.kind === "A") {
      const { v } = $tt_m;
      if (v > 0) {
        $tt_v0$r = 1;
        break;
      }
    }
    if ($tt_m.kind === "A") {
      $tt_v0$r = 0;
      break;
    }
    $tt_v0$r = -1;
    break;
  } while (false);
}
const r = $tt_v0$r;
\ No newline at end of file
