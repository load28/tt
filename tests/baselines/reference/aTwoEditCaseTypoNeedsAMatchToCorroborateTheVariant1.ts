//// [aTwoEditCaseTypoNeedsAMatchToCorroborateTheVariant1.tt] ////
variant Shape { Circle(radius: number), Empty }
const a = match (s) { Cyrcla(radius) => radius, Empty => 0 };

