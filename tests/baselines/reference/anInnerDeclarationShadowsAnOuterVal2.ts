//// [anInnerDeclarationShadowsAnOuterVal2.tt] ////
val const cfg = { a: 1 };
function f(cfg: C) {
  cfg.a = 2;
}


//// [anInnerDeclarationShadowsAnOuterVal2.ts]
const cfg = { a: 1 };
function f(cfg: C) {
  cfg.a = 2;
}
