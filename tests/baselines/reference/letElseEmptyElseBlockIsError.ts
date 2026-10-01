//// [letElseEmptyElseBlockIsError.tt] ////
function f(): number {
  const Some(v) = find() else { };
  return v;
}

