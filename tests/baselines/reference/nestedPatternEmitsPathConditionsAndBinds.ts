//// [nestedPatternEmitsPathConditionsAndBinds.tt] ////

const n = match (r) {
  Ok(value: Some(value: v)) => v,
  Ok(value: None()) => 0,
  _ => -1,
};


//// [nestedPatternEmitsPathConditionsAndBinds.ts]

let $tt_v0$n;
{
  const $tt_m = r;
  do {
    if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some") {
      const { value: v } = $tt_m.value;
      $tt_v0$n = v;
      break;
    }
    if ($tt_m.kind === "Ok" && $tt_m.value.kind === "None") {
      $tt_v0$n = 0;
      break;
    }
    $tt_v0$n = -1;
    break;
  } while (false);
}
const n = $tt_v0$n;
