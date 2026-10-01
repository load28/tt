//// [nestedPatternWithGuard.tt] ////

const n = match (r) {
  Ok(value: Some(value: v)) if v > 0 => v,
  _ => 0,
};


//// [nestedPatternWithGuard.ts]

let $tt_v0$n;
{
  const $tt_m = r;
  do {
    if ($tt_m.kind === "Ok" && $tt_m.value.kind === "Some") {
      const { value: v } = $tt_m.value;
      if (v > 0) {
        $tt_v0$n = v;
        break;
      }
    }
    $tt_v0$n = 0;
    break;
  } while (false);
}
const n = $tt_v0$n;
