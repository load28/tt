//// [orPatternBindingMismatchIsError4.tt] ////
const r = match (x) { A(v) | B(v, w) => v, _ => 0 };

