//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral3.tt] ////
function f(): number {
  const Some(v) = find() else { return match (o) { A => 1, _ => 0 }; };
  return v;
}


//// [letElseDivergesWhenTheReturnValueIsAnObjectLiteral3.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    let $tt_v0: number;
    {
      const $tt_m = o;
      switch ($tt_m.kind) {
        case "A": {
          $tt_v0 = 1;
          break;
        }
        default: {
          $tt_v0 = 0;
          break;
        }
      }
    }
    return $tt_v0;
  }
  const { v } = $tt_t0;
  return v;
}
