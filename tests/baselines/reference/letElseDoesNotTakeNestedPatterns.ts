//// [letElseDoesNotTakeNestedPatterns.tt] ////
function f() {
  const Some(value: Ok(v)) = g() else { return; };
}

