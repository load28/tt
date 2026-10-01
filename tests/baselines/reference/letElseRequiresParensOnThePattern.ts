//// [letElseRequiresParensOnThePattern.tt] ////
function f(): number {
  const Point = find() else { return 0; };
  return 1;
}

