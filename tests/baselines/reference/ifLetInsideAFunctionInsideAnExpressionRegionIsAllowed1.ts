//// [ifLetInsideAFunctionInsideAnExpressionRegionIsAllowed1.tt] ////
const v = match (run(() => { if let A(x) = e { return x; } return 0; })) {
  Ok(value) => value,
  _ => 0,
};


//// [ifLetInsideAFunctionInsideAnExpressionRegionIsAllowed1.ts]
let $tt_v0$v;
{
  const $tt_m = (run(() => { {
    const $tt_t0 = e;
    if ($tt_t0.kind === "A") {
      const { x } = $tt_t0;
      return x;
    }
  } return 0; }));
  switch ($tt_m.kind) {
    case "Ok": {
      const { value } = $tt_m;
      $tt_v0$v = value;
      break;
    }
    default: {
      $tt_v0$v = 0;
      break;
    }
  }
}
const v = $tt_v0$v;
