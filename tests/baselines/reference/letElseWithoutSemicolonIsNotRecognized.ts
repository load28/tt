//// [letElseWithoutSemicolonIsNotRecognized.tt] ////
function f(): number {
  const Some(v) = find() else { return 0; }
  return v;
}

