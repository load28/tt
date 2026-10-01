//// [tupleMatchBareWildcardArmSkipsTheCheckAndMustBeLast2.tt] ////
const r = match (a, b) {
  _ => 0,
  (A, B) => 1,
};

