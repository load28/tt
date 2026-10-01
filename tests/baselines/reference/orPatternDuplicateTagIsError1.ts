//// [orPatternDuplicateTagIsError1.tt] ////
const r = match (x) { A | A => 1, _ => 0 };

