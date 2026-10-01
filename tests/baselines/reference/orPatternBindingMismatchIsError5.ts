//// [orPatternBindingMismatchIsError5.tt] ////
const r = match (x) { A(v) | B(_) => v, _ => 0 };

