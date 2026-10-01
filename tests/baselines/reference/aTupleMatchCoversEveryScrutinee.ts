//// [aTupleMatchCoversEveryScrutinee.tt] ////
variant S { A(x: number), B }
variant T { C(), D }
const v = match (s, t) { (A(x), C) => x };

