//// [orPatternBindingMismatchIsError2.tt] ////
const r = match (x) { A(v) | B(v: w) => w, _ => 0 };

