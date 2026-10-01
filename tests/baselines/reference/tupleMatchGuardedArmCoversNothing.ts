//// [tupleMatchGuardedArmCoversNothing.tt] ////

variant Coin { Heads(), Tails }
const r = match (a, b) {
  (Heads, Heads) if lucky() => 1,
  (Heads, Heads) => 2,
  (Heads, Tails) => 3,
  (Tails, Heads) => 4,
};

