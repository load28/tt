//// [nestedPatternMixesPlainBindingsAtEachLevel.tt] ////

const n = match (r) {
  Both(left, right: Some(value)) => left + value,
  _ => 0,
};


//// [nestedPatternMixesPlainBindingsAtEachLevel.ts]

let $tt_v0$n;
{
  const $tt_m = r;
  do {
    if ($tt_m.kind === "Both" && $tt_m.right.kind === "Some") {
      const { left } = $tt_m;
      const { value } = $tt_m.right;
      $tt_v0$n = left + value;
      break;
    }
    $tt_v0$n = 0;
    break;
  } while (false);
}
const n = $tt_v0$n;
