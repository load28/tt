//// [invalidTypescriptInsideAClaimedConstructIsALocatedError.tt] ////
const r = result {
  const a = try f();
  const b = ;
  return a;
};

