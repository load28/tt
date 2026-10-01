//// [misspelledCaseInAMatchArmNamesTheCaseMeant.tt] ////
variant Shape { Circle(radius: number), Empty }
const a = match (s) {
  Circel(radius) => radius,
  Empty => 0,
};

