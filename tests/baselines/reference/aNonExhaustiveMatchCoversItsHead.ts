//// [aNonExhaustiveMatchCoversItsHead.tt] ////
variant S { A(x: number), B }
const v = match (s) { A(x) => x };

