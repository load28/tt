//// [tupleMatchScrutineesEvaluateOnceEachLeftToRight.tt] ////

variant Coin { Heads(), Tails }
const order: string[] = [];
function heads(name: string): Coin { order.push(name); return Coin.Heads(); }
const r = match (heads("a"), heads("b")) {
  (Heads, Heads) => 1,
  _ => 0,
};
console.log(order.join(","), r);

export {};


//// [tupleMatchScrutineesEvaluateOnceEachLeftToRight.ts]

type Coin =
  | { kind: "Heads" }
  | { kind: "Tails" };
const Coin = {
  Heads: (): Coin => ({ kind: "Heads" }),
  Tails: { kind: "Tails" } as const,
};
const order: string[] = [];
function heads(name: string): Coin { order.push(name); return Coin.Heads(); }
let $tt_v0: number;
{
  const $tt_m0 = heads("a");
  const $tt_m1 = heads("b");
  do {
    if ($tt_m0.kind === "Heads" && $tt_m1.kind === "Heads") {
      $tt_v0 = 1;
      break;
    }
    $tt_v0 = 0;
    break;
  } while (false);
}
const r = $tt_v0;
console.log(order.join(","), r);

export {};
