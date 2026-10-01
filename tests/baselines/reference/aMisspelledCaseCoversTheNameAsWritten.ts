//// [aMisspelledCaseCoversTheNameAsWritten.tt] ////
variant Shape { Circle(r: number), Square(s: number) }
const v = match (s) { Circel(r) => r, Square(s) => s };

