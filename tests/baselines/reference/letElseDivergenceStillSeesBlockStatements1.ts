//// [letElseDivergenceStillSeesBlockStatements1.tt] ////
function f(): number {
  const Some(v) = find() else { if (c) { log("x"); } return 0; };
  return v;
}


//// [letElseDivergenceStillSeesBlockStatements1.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    if (c) { log("x"); } return 0;
  }
  const { v } = $tt_t0;
  return v;
}
