//// [valParameterPositionsBeyondPlainIdentifiers3.tt] ////
try {
  f();
} catch (val error: any) {
  error.code = 1;
}

