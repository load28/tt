//// [valOnALetElsePatternCoversTheNamesItBinds.tt] ////
variant Opt { Some(value: Box), None }
function f(o: Opt) {
  val const Some(value) = o else { return; };
  value.n = 1;
}

