//// [valParameterPositionsBeyondPlainIdentifiers1.tt] ////
function foo(val { user }: Ctx) {
  user.name = "x";
}

