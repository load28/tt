//// [aMissingArmCommaIsOnlyAParseError.tt] ////
variant O { A, B, C }
declare const o: O;
export const r = match (o) {
  A => 1
  B => 2,
  C => 3,
};

