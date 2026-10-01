//// [letElseNonDivergingElseEndingInABraceIsStillAnError3.tt] ////
function f(): number {
  const Some(v) = find() else { for (const x of xs) { log(x); } };
  return v;
}

