//// [valCoversEveryOrPatternAlternativesBindings.tt] ////
variant E { A(x: Box), B(x: Box) }
function f(e: E) {
  val const A(x) | B(x) = e else { return; };
  x.n = 1;
}

