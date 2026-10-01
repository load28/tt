//// [malformedIfLetIsAnErrorWithPosition2.tt] ////
function f() {
  if let Some(v) = o { g(); } else if (x) { h(); }
}

