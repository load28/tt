//// [aWitnessNamesTheValueThatIsMissing.tt] ////
variant Inner { Yes(n: number), No }
variant Outer { Wrap(inner: Inner), Bare }
const a = match (o) { Wrap(inner: Yes(n)) => n, Bare => -1 };

