//// [orPatternBindingMismatchIsError1.tt] ////
const r = match (x) { A(v) | B(w) => v, _ => 0 };

