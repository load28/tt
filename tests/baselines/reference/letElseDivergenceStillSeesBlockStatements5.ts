//// [letElseDivergenceStillSeesBlockStatements5.tt] ////
function f(): number {
  const Some(v) = find() else { const g = () => { return 1; }; return g(); };
  return v;
}


//// [letElseDivergenceStillSeesBlockStatements5.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    const g = () => { return 1; }; return g();
  }
  const { v } = $tt_t0;
  return v;
}
