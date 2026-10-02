//// [inertArgumentsAreNotCapturedOutOfTheirContextualPosition1.tt] ////
const items = [{run: (n) => n}, match (x) { A(v) => v, _ => 0 }];


//// [inertArgumentsAreNotCapturedOutOfTheirContextualPosition1.ts]
let $tt_v0$items;
{
  const $tt_m = x;
  switch ($tt_m.kind) {
    case "A": {
      const { v } = $tt_m;
      $tt_v0$items = v;
      break;
    }
    default: {
      $tt_v0$items = 0;
      break;
    }
  }
}
const items = [{run: (n) => n}, $tt_v0$items];
\ No newline at end of file
