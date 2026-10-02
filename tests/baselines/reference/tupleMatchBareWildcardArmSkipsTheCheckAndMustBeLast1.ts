//// [tupleMatchBareWildcardArmSkipsTheCheckAndMustBeLast1.tt] ////

variant Coin { Heads(), Tails }
const r = match (a, b) {
  (Heads, Heads) => 1,
  _ => 0,
};


//// [tupleMatchBareWildcardArmSkipsTheCheckAndMustBeLast1.ts]

type Coin =
  | { kind: "Heads" }
  | { kind: "Tails" };
const Coin = {
  Heads: (): Coin => ({ kind: "Heads" }),
  Tails: { kind: "Tails" } as const,
};
let $tt_v0$r: number;
{
  const $tt_m0 = a;
  const $tt_m1 = b;
  do {
    if ($tt_m0.kind === "Heads" && $tt_m1.kind === "Heads") {
      $tt_v0$r = 1;
      break;
    }
    $tt_v0$r = 0;
    break;
  } while (false);
}
const r = $tt_v0$r;
