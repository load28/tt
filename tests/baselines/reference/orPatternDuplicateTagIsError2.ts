//// [orPatternDuplicateTagIsError2.tt] ////
const r = match (x) { A | B => 1, B => 2, _ => 0 };

