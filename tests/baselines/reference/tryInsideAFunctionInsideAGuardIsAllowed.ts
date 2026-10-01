//// [tryInsideAFunctionInsideAGuardIsAllowed.tt] ////
const r = match (x) {
  A(v) if run(() => { try g(); return true; }) => v,
  _ => 0,
};


//// [tryInsideAFunctionInsideAGuardIsAllowed.ts]
let $tt_v0$r;
{
  const $tt_m = x;
  do {
    if ($tt_m.kind === "A") {
      const { v } = $tt_m;
      if (run(() => { const $tt_t0 = g();
      if (!("value" in $tt_t0)) {
        return $tt_t0;
      } return true; })) {
        $tt_v0$r = v;
        break;
      }
    }
    $tt_v0$r = 0;
    break;
  } while (false);
}
const r = $tt_v0$r;
