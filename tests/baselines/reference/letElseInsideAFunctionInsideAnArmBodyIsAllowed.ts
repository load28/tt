//// [letElseInsideAFunctionInsideAnArmBodyIsAllowed.tt] ////
const x = match (r) {
  Ok(value) => { const f = () => { const Some(v) = h(value) else { return 0; }; return v; }; return f(); },
  _ => 0,
};


//// [letElseInsideAFunctionInsideAnArmBodyIsAllowed.ts]
let $tt_v0$x;
{
  const $tt_m = r;
  switch ($tt_m.kind) {
    case "Ok": {
      const { value } = $tt_m;
      const f = () => { const $tt_t0 = h(value);
      if ($tt_t0.kind !== "Some") {
        return 0;
      }
      const { v } = $tt_t0; return v; }; $tt_v0$x = f(); break;
    }
    default: {
      $tt_v0$x = 0;
      break;
    }
  }
}
const x = $tt_v0$x;
