//// [orPatternBindingMismatchIsError3.tt] ////
const r = match (x) { A | B(v) => 1, _ => 0 };

