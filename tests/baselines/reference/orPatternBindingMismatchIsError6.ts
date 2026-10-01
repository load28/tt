//// [orPatternBindingMismatchIsError6.tt] ////
const r = match (x) { A(v) | B(w: v) => v, _ => 0 };

