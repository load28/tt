//// [misspelledFieldNamesTheFieldMeant.tt] ////
variant Shape { Circle(radius: number), Empty }
const a = match (s) { Circle(radiuz) => radiuz, Empty => 0 };

