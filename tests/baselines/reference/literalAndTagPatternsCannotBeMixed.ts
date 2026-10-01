//// [literalAndTagPatternsCannotBeMixed.tt] ////
const v = match (x) { Some(v) => v, "none" => 0 };

