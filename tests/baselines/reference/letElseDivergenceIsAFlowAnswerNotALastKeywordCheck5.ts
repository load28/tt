//// [letElseDivergenceIsAFlowAnswerNotALastKeywordCheck5.tt] ////
function f(): number {
  const Some(v) = find() else { return 0; log("never"); };
  return v;
}


//// [letElseDivergenceIsAFlowAnswerNotALastKeywordCheck5.ts]
function f(): number {
  const $tt_t0 = find();
  if ($tt_t0.kind !== "Some") {
    return 0; log("never");
  }
  const { v } = $tt_t0;
  return v;
}
