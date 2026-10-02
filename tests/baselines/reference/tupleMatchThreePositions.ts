//// [tupleMatchThreePositions.tt] ////

variant B { T(), F }
const r = match (x, y, z) {
  (T, _, _) => 1,
  (F, T, _) => 2,
  (F, F, T) => 3,
};

