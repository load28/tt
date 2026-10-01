//// [tupleMatchBindsFieldsFromEachPosition.tt] ////

const r = match (a, b) {
  (Some(value: x), Some(value: y)) => x + y,
  _ => 0,
};


//// [tupleMatchBindsFieldsFromEachPosition.ts]

let $tt_v0$r;
{
  const $tt_m0 = a;
  const $tt_m1 = b;
  do {
    if ($tt_m0.kind === "Some" && $tt_m1.kind === "Some") {
      const { value: x } = $tt_m0;
      const { value: y } = $tt_m1;
      $tt_v0$r = x + y;
      break;
    }
    $tt_v0$r = 0;
    break;
  } while (false);
}
const r = $tt_v0$r;
