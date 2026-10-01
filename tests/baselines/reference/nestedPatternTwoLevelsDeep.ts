//// [nestedPatternTwoLevelsDeep.tt] ////

const n = match (r) {
  Ok(value: Some(value: Pair(a, b))) => a + b,
  _ => 0,
};


//// [nestedPatternTwoLevelsDeep.ts]

let $tt_v0$n;
{
  const $tt_m = r;
  do {
    if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some" && $tt_m.value.value.kind === "Pair") {
      const { a, b } = $tt_m.value.value;
      $tt_v0$n = a + b;
      break;
    }
    $tt_v0$n = 0;
    break;
  } while (false);
}
const n = $tt_v0$n;
