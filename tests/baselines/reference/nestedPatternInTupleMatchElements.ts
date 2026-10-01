//// [nestedPatternInTupleMatchElements.tt] ////

const n = match (a, b) {
  (Ok(value: Some(value: x)), Ok(value: Some(value: y))) => x + y,
  _ => 0,
};


//// [nestedPatternInTupleMatchElements.ts]

let $tt_v0$n;
{
  const $tt_m0 = a;
  const $tt_m1 = b;
  do {
    if ($tt_m0.kind === "Ok" && $tt_m0.value.kind === "Some" && $tt_m1.kind === "Ok" && $tt_m1.value.kind === "Some") {
      const { value: x } = $tt_m0.value;
      const { value: y } = $tt_m1.value;
      $tt_v0$n = x + y;
      break;
    }
    $tt_v0$n = 0;
    break;
  } while (false);
}
const n = $tt_v0$n;
