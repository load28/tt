//// [letElseDivergenceStillSeesBlockStatements6.tt] ////
function f(): number {
  const Some(v) = find() else { const o = { n: 1 }; return o.n; };
  return v;
}


//// [letElseDivergenceStillSeesBlockStatements6.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    const o = { n: 1 }; return o.n;
  }
  const { v } = $tt_t0;
  return v;
}
