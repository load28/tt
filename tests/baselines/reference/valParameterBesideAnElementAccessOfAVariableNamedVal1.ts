//// [valParameterBesideAnElementAccessOfAVariableNamedVal1.tt] ////
const g = c ? (val [0]) : w => w;
function read(val [user]: User[]) {
  user.name = "x";
}

