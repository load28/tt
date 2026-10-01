//// [ifLetInsideAFunctionInsideAnExpressionRegionIsAllowed2.tt] ////
const s = `${run(() => { if let A(x) = e { log(x); } return 1; })}`;


//// [ifLetInsideAFunctionInsideAnExpressionRegionIsAllowed2.ts]
const s = `${run(() => { {
  const $tt_t0 = e;
  if ($tt_t0.kind === "A") {
    const { x } = $tt_t0;
    log(x);
  }
} return 1; })}`;
