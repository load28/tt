//// [letElseDivergenceStillSeesBlockStatements2.tt] ////
function f(): number {
  const Some(v) = find() else { try { log("x"); } catch (e) { log("y"); } return 0; };
  return v;
}


//// [letElseDivergenceStillSeesBlockStatements2.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    try { log("x"); } catch (e) { log("y"); } return 0;
  }
  const { v } = $tt_t0;
  return v;
}
