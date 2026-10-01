//// [oneElementTuplePatternReportsTheExactArity1.tt] ////
const r = match (a, b) {
  (A) => 1,
  _ => 0,
};

