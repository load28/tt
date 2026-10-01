//// [letElseDivergenceStillSeesBlockStatements3.tt] ////
function f(): number {
  const Some(v) = find() else { for (const x of xs) { log(x); } return 0; };
  return v;
}


//// [letElseDivergenceStillSeesBlockStatements3.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    for (const x of xs) { log(x); } return 0;
  }
  const { v } = $tt_t0;
  return v;
}
