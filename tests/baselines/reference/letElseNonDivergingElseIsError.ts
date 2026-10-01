//// [letElseNonDivergingElseIsError.tt] ////
function f(): number {
  const Some(v) = find() else { log(); };
  return v;
}

