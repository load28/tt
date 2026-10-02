//// [letElseDivergenceIsAFlowAnswerNotALastKeywordCheck3.tt] ////
function f(): number {
  const Some(v) = find() else { if (c) return 1; else return 2; };
  return v;
}


//// [letElseDivergenceIsAFlowAnswerNotALastKeywordCheck3.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    if (c) return 1; else return 2;
  }
  const { v } = $tt_t0;
  return v;
}
