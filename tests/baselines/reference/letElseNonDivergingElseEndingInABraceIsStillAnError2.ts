//// [letElseNonDivergingElseEndingInABraceIsStillAnError2.tt] ////
function f(): number {
  const Some(v) = find() else { if (c) { return 1; } };
  return v;
}

