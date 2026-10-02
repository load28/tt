//// [letElseExpressionMayBeAMatch.tt] ////
function f(): number {
  const Some(v) = match (x) { A => some(1), _ => none() } else { return 0; };
  return v;
}


//// [letElseExpressionMayBeAMatch.ts]
function f(): number {
  let $tt_t0; {
    const $tt_m = x;
    switch ($tt_m.kind) {
      case "A": {
        $tt_t0 = some(1);
        break;
      }
      default: {
        $tt_t0 = none();
        break;
      }
    }
  }
  if ($tt_t0.kind !== "Some") {
    return 0;
  }
  const { v } = $tt_t0;
  return v;
}
