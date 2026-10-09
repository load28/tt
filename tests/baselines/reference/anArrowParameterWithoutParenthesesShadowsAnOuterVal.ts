//// [anArrowParameterWithoutParenthesesShadowsAnOuterVal.tt] ////
val const x = { a: 1 };
[{ a: 1 }].forEach(x => { x.a = 1; });
[{ a: 1 }].forEach(x => x.a = 1);
[{ a: 1 }].forEach(async x => { x.a = 1; });
[{ a: 1 }].forEach(y => { x.a = 2; });
export {};

