//// [valParameterBesideAnElementAccessOfAVariableNamedVal2.tt] ////
const g = c ? (val [0]) : w => w;
f(val [0])
{
  val.name = 1;
}


//// [valParameterBesideAnElementAccessOfAVariableNamedVal2.ts]
const g = c ? (val [0]) : w => w;
f(val [0])
{
  val.name = 1;
}
