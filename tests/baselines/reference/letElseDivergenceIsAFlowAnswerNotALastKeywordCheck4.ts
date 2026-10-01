//// [letElseDivergenceIsAFlowAnswerNotALastKeywordCheck4.tt] ////
function f(): number {
  const Some(v) = find() else { { return 1; } };
  return v;
}


//// [letElseDivergenceIsAFlowAnswerNotALastKeywordCheck4.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    { return 1; }
  }
  const { v } = $tt_t0;
  return v;
}
