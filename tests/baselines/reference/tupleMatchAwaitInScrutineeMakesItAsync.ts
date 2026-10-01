//// [tupleMatchAwaitInScrutineeMakesItAsync.tt] ////
async function f() {
  return match (await a, b) {
    (X, Y) => 1,
    _ => 0,
  };
}


//// [tupleMatchAwaitInScrutineeMakesItAsync.ts]
async function f() {
  let $tt_v0: number;
  {
    const $tt_m0 = await a;
    const $tt_m1 = b;
    do {
      if ($tt_m0.kind === "X" && $tt_m1.kind === "Y") {
        $tt_v0 = 1;
        break;
      }
      $tt_v0 = 0;
      break;
    } while (false);
  }
  return $tt_v0;
}
