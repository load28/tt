//// [letElseNonDivergingElseEndingInABraceIsStillAnError1.tt] ////
function f(): number {
  const Some(v) = find() else { const o = { n: 1 }; };
  return v;
}

