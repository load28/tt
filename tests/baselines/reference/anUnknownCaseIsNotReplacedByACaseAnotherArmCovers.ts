//// [anUnknownCaseIsNotReplacedByACaseAnotherArmCovers.tt] ////
variant S { Alpha(x: number), Beta(y: string), Gamma }
declare const s: S;
export const covered = match (s) { Alpha(x) => x, Beta(y) => y.length, Gamma => 0, Delta => 1 };
export const open = match (s) { Alpha(x) => x, Bta(y) => y.length, Gamma => 0 };
export const guarded = match (s) {
  Alpha(x) => x,
  Beta(y) if y.length > 0 => 1,
  Bta(y) => 2,
  Gamma => 0,
};

