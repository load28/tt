//// [anInnerDeclarationShadowsAnOuterVal1.tt] ////
val const x = { a: 1 };
{
  const x = { a: 2 };
  x.a = 3;
}


//// [anInnerDeclarationShadowsAnOuterVal1.ts]
const x = { a: 1 };
{
  const x = { a: 2 };
  x.a = 3;
}
