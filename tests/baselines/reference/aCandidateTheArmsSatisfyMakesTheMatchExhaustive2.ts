//// [aCandidateTheArmsSatisfyMakesTheMatchExhaustive2.tt] ////
variant Big { A(s: string), B, C, D }
variant Small { A(s: string), B, C }
const f = (v: Small) => match (v) { A(s) => s, B => "b" };

