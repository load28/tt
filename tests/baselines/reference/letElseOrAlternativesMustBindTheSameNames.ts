//// [letElseOrAlternativesMustBindTheSameNames.tt] ////
variant E { A(x: number), B(y: number), C }
function f(e: E): number {
  const A(x) | B(y) = e else { return 0; };
  return x;
}

