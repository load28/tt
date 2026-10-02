//// [literalDuplicateAcrossOrAlternativesIsError.tt] ////
const v = match (x) { "a" | "b" => 1, "b" | "c" => 2 };

