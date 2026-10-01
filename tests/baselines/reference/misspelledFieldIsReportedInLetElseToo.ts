//// [misspelledFieldIsReportedInLetElseToo.tt] ////
variant Shape { Circle(radius: number), Empty }
function f(): number {
  const Circle(radiuz) = s else { return 0; };
  return radiuz;
}

