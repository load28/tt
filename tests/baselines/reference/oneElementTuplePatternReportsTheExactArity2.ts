//// [oneElementTuplePatternReportsTheExactArity2.tt] ////
const r = match (a) {
  (A, B) => 1,
  _ => 0,
};

